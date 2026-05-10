use anyhow::Context;
use anyhow::Result;
use axum::{
    extract::{
        ws::{WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};
use tracing::warn;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod feeds;
mod jupiter_perps;
mod jupiter_prediction;
mod magicblock;
mod orderbook;
mod perps;
mod polymarket;
mod risk;
mod utils;
mod wallet;
mod ws;

use crate::utils::BlockhashCache;
use crate::ws::ServerMessage;
use config::EngineConfig;
use jupiter_prediction::JupiterPredictionBridge;
use orderbook::OrderbookEngine;
use perps::PerpsEngine;
use polymarket::PolymarketBridge;
use risk::RiskEngine;
use solana_sdk::hash::Hash;

/// Shared engine state across all WebSocket connections
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<EngineConfig>,
    pub orderbook: Arc<OrderbookEngine>,
    pub perps: Arc<PerpsEngine>,
    pub polymarket: Arc<PolymarketBridge>,
    pub jupiter_prediction: Arc<JupiterPredictionBridge>,
    pub risk: Arc<RiskEngine>,
    pub broadcast_tx: broadcast::Sender<ServerMessage>,
    pub blockhash_cache: Arc<BlockhashCache>,
    pub er_tx_builder: Arc<magicblock::ErTxBuilder>,
    pub session_registry: Arc<magicblock::SessionRegistry>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "soldex_engine=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    dotenvy::dotenv().ok();
    let config = Arc::new(EngineConfig::from_env()?);

    info!("🚀 Soldex Engine starting on {}", config.listen_addr);

    let (broadcast_tx, _) = broadcast::channel::<ServerMessage>(1024);

    let orderbook = Arc::new(OrderbookEngine::new(config.clone()));
    let perps = Arc::new(PerpsEngine::new(config.clone()).await?);
    let polymarket = Arc::new(PolymarketBridge::new(config.clone()));
    let jupiter_prediction = Arc::new(JupiterPredictionBridge::new(config.clone()));
    let risk = Arc::new(RiskEngine::new());
    let initial_hash = Hash::default();
    let blockhash_cache = Arc::new(BlockhashCache::new(initial_hash));

    // ── MagicBlock ER setup ───────────────────────────────────────────────────

    // 1. Load keeper keypair via WalletManager (AES-GCM encrypted)
    let wallet_manager = wallet::WalletManager::new(&config.wallet_encryption_key)?;

    let keeper_enc_path = config.keeper_keypair_path.replace(".json", ".enc");

    let keeper_keypair: Arc<solana_sdk::signature::Keypair> =
        if std::path::Path::new(&keeper_enc_path).exists() {
            let pubkey = wallet_manager
                .load_keypair_from_file(&keeper_enc_path)
                .await?;
            wallet_manager.get_keypair(&pubkey).await?
        } else if std::path::Path::new(&config.keeper_keypair_path).exists() {
            warn!(
                "Plain {} found — migrating to encrypted {}",
                config.keeper_keypair_path, keeper_enc_path
            );
            let bytes = std::fs::read_to_string(&config.keeper_keypair_path)?;
            let raw: Vec<u8> = serde_json::from_str(&bytes)?;
            let keypair = solana_sdk::signature::Keypair::from_bytes(&raw)
                .map_err(|e| anyhow::anyhow!("invalid keeper.json: {e}"))?;
            let pubkey = wallet_manager.import_keypair("keeper", keypair).await?;
            wallet_manager
                .save_keypair_to_file(&pubkey, &keeper_enc_path)
                .await?;
            std::fs::remove_file(&config.keeper_keypair_path)?;
            info!("Keeper keypair encrypted and saved to {}", keeper_enc_path);
            wallet_manager.get_keypair(&pubkey).await?
        } else {
            // no keypair found — ephemeral for dev only            warn!("No keeper keypair found — using ephemeral keypair for dev");
            Arc::new(solana_sdk::signature::Keypair::new())
        };

    // 2. TEE authentication
    let tee_client = Arc::new(magicblock::TeeClient::new(
        keeper_keypair.clone(),
        config.tee_auth_url.clone(),
        config.tee_rpc_url.clone(),
    ));
    tee_client.authenticate().await?;
    info!("✅ TEE session established");

    let er_tx_builder = Arc::new(magicblock::ErTxBuilder::new(
        &config,
        keeper_keypair.clone(),
    ));

    let session_registry = magicblock::SessionRegistry::new(
        keeper_keypair.clone(),
        config.tee_auth_url.clone(),
        config.tee_rpc_url.clone(),
        config.er_rpc_url.clone(),
        config.er_ws_url.clone(),
        config.solana_rpc_url.clone(),
    );
    let keeper_bot = Arc::new(magicblock::KeeperBot::new(
        session_registry.clone(),
        er_tx_builder.clone(),
        blockhash_cache.clone(),
        magicblock::keeper::KeeperConfig {
            funding_interval_secs: config.funding_interval_secs,
            liquidation_scan_interval_secs: config.liquidation_scan_interval_secs,
            min_margin_ratio_bps: 500,
        },
        perps.clone(),
    ));

    // Spawn price updater — keeps last_mark_price fresh for all delegated positions
    tokio::spawn({
        let registry = session_registry.clone();
        let rx = broadcast_tx.subscribe();
        async move {
            magicblock::run_price_updater(registry, rx).await;
        }
    });

    // Fetch MarketState accounts from chain and register with keeper
    let rpc = Arc::new(
        solana_client::nonblocking::rpc_client::RpcClient::new_with_commitment(
            config.solana_rpc_url.clone(),
            solana_sdk::commitment_config::CommitmentConfig::confirmed(),
        ),
    );
    let program_id: solana_sdk::pubkey::Pubkey = config
        .program_id
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid program_id: {e}"))?;

    magicblock::register_markets(keeper_bot.clone(), rpc.clone(), &program_id, perps.clone())
        .await?;
    let markets = keeper_bot.list_markets();
    // Delegation program ID (MagicBlock)
    let delegation_program_id: solana_sdk::pubkey::Pubkey =
        "DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh"
            .parse()
            .unwrap();

    for (market_id, market_pda) in &markets {
        // Derive delegation record PDA
        let (delegation_record, _) = solana_sdk::pubkey::Pubkey::find_program_address(
            &[b"delegation", market_pda.as_ref()],
            &delegation_program_id,
        );

        // Skip if already delegated
        match rpc.get_account(&delegation_record).await {
            Ok(_) => {
                info!(
                    "Market {} already delegated — skipping",
                    hex::encode(market_id)
                );
                continue;
            }
            Err(_) => {
                // Not delegated yet — proceed
            }
        }

        let blockhash = rpc.get_latest_blockhash().await?;
        let tx = er_tx_builder.delegate_market_tx(market_pda, *market_id, blockhash)?;
        rpc.send_and_confirm_transaction(&tx)
            .await
            .with_context(|| {
                format!(
                "delegate_market failed for {} — keeper pubkey must match market.admin on devnet",
                hex::encode(market_id)
            )
            })?;
        info!("✅ Market {} delegated to ER", hex::encode(market_id));
    }

    // Pre-warm a keeper session so funding ticks can fire without waiting for a user
    {
        use solana_sdk::signer::Signer;
        let keeper_pubkey = keeper_keypair.pubkey();
        if let Err(e) = session_registry.get_or_create(&keeper_pubkey).await {
            warn!("Keeper session pre-warm failed: {e} — funding ticks will wait for first user");
        } else {
            info!("✅ Keeper ER session pre-warmed");
        }
    }

    info!(
        "✅ MagicBlock ER session connected to {}",
        config.er_rpc_url
    );

    // spawn keeper after markets are registered and session is ready
    tokio::spawn({
        let bot = keeper_bot.clone();
        async move { bot.run().await }
    });

    // ─────────────────────────────────────────────────────────────────────────

    let state = AppState {
        config: config.clone(),
        orderbook: orderbook.clone(),
        perps: perps.clone(),
        polymarket: polymarket.clone(),
        jupiter_prediction: jupiter_prediction.clone(),
        risk: risk.clone(),
        broadcast_tx: broadcast_tx.clone(),
        blockhash_cache: blockhash_cache.clone(),
        er_tx_builder: er_tx_builder.clone(),
        session_registry: session_registry.clone(),
    };

    // blockhash cache refresh loop
    let cache = blockhash_cache.clone();
    let perps_clone = perps.clone();

    tokio::spawn(async move {
        loop {
            // Use your existing RPC (PerpsEngine already has RpcClient)
            let hash = perps_clone.get_latest_blockhash().await;

            match hash {
                Ok(h) => {
                    cache.set(h).await;
                }
                Err(e) => {
                    error!("Blockhash fetch failed: {}", e);
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        }
    });

    // Jupiter Perps feed
    let tx_clone = broadcast_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = jupiter_perps::start_jupiter_feed(tx_clone).await {
            error!("Jupiter Perps feed error: {e}");
        }
    });

    let tx_clone = broadcast_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = jupiter_perps::start_jupiter_stats_poller(tx_clone).await {
            error!("Jupiter stats poller error: {e}");
        }
    });

    // Spawn Polymarket feeds
    let pm_bridge = polymarket.clone();
    let tx_clone = broadcast_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = feeds::start_polymarket_feed(pm_bridge, tx_clone).await {
            error!("Polymarket opportunity feed error: {e}");
        }
    });

    let pm_bridge = polymarket.clone();
    let tx_clone = broadcast_tx.clone();
    tokio::spawn(async move {
        use crate::polymarket::start_polymarket_ws_feed;
        if let Err(e) = start_polymarket_ws_feed(pm_bridge, tx_clone).await {
            error!("Polymarket WS feed error: {e}");
        }
    });

    // Spawn Jupiter Prediction feed
    println!(">>> DEBUG: SPAWNING JUPITER PREDICTION FEED");
    let jp_bridge = jupiter_prediction.clone();
    let tx_clone = broadcast_tx.clone();
    tokio::spawn(async move {
        println!(">>> DEBUG: INSIDE tokio::spawn FOR JUPITER PREDICTION");
        if let Err(e) = feeds::start_jupiter_prediction_feed(jp_bridge, tx_clone).await {
            error!("Jupiter Prediction opportunity feed error: {e}");
        }
    });

    // Bridge broadcast ticker updates → perps.tickers (keeper reads from here)
    let perps_bridge = perps.clone();
    let mut ticker_rx = broadcast_tx.subscribe();
    tokio::spawn(async move {
        while let Ok(msg) = ticker_rx.recv().await {
            if let ServerMessage::TickerUpdate { ticker } = msg {
                perps_bridge
                    .tickers
                    .write()
                    .await
                    .insert(ticker.market_id.clone(), ticker);
            }
        }
    });

    // Spawn Perps market data feed (fills tickers cache — keeper depends on this)
    let perps_feed = perps.clone();
    let tx_clone = broadcast_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = perps_feed.run_market_data_feed(tx_clone).await {
            error!("Perps market data feed error: {e}");
        }
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/health", get(health_handler))
        .layer(cors)
        .with_state(state);

    let addr = config.listen_addr.parse().expect("Invalid listen address");
    info!("✅ Engine listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

async fn health_handler() -> impl IntoResponse {
    axum::Json(serde_json::json!({ "status": "ok", "engine": "soldex-v0.1" }))
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    ws::handle_client(socket, state).await;
}
