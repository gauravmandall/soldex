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
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use solana_sdk::signature::read_keypair_file;
use tracing::warn;

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
use orderbook::OrderbookEngine;
use perps::PerpsEngine;
use jupiter_prediction::JupiterPredictionBridge;
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
}

#[tokio::main]
async fn main() -> Result<()> {
    // Init tracing
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
    let initial_hash = Hash::default(); // temporary
    let blockhash_cache = Arc::new(BlockhashCache::new(initial_hash));
    // ── MagicBlock ER setup ───────────────────────────────────────────────────
    let keeper_keypair = Arc::new(
        solana_sdk::signature::read_keypair_file(&config.keeper_keypair_path).unwrap_or_else(
            |_| {
                // No keeper keypair found — generate ephemeral one for dev
                warn!(
                    "keeper keypair not found at {} — using ephemeral keypair",
                    config.keeper_keypair_path
                );
                solana_sdk::signature::Keypair::new()
            },
        ),
    );

    let er_session = magicblock::ErSession::new(
        &config.er_rpc_url,
          &config.er_ws_url,
        &config.solana_rpc_url,
        keeper_keypair.clone(),
    )
    .await?;

    let er_tx_builder = Arc::new(magicblock::ErTxBuilder::new(
        &config,
        keeper_keypair.clone(),
    ));

    let keeper_bot = Arc::new(magicblock::KeeperBot::new(
        er_session.clone(),
        er_tx_builder.clone(),
        blockhash_cache.clone(),
        magicblock::keeper::KeeperConfig {
            funding_interval_secs: config.funding_interval_secs,
            liquidation_scan_interval_secs: config.liquidation_scan_interval_secs,
            min_margin_ratio_bps: 500,
        },
    ));

    // Spawn keeper bot — runs funding + liquidation loops in background
    tokio::spawn({
        let bot = keeper_bot.clone();
        async move { bot.run().await }
    });

    info!(
        "✅ MagicBlock ER session connected to {}",
        config.er_rpc_url
    );
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
    };

    // Spawn Jupiter Perps feed

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

    // Spawn Hyperliquid feed
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

    let pm_bridge = polymarket.clone();
    let tx_clone = broadcast_tx.clone();
    // TODO: start_polymarket_ws_feed — needs implementation in polymarket/mod.rs
    drop(pm_bridge);
    drop(tx_clone);

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
