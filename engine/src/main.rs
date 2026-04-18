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

mod config;
mod orderbook;
mod feeds;
mod perps;
mod polymarket;
mod risk;
mod wallet;
mod ws;
mod hyperliquid;

use config::EngineConfig;
use orderbook::OrderbookEngine;
use perps::PerpsEngine;
use polymarket::PolymarketBridge;
use risk::RiskEngine;
use crate::ws::ServerMessage;

/// Shared engine state across all WebSocket connections
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<EngineConfig>,
    pub orderbook: Arc<OrderbookEngine>,
    pub perps: Arc<PerpsEngine>,
    pub polymarket: Arc<PolymarketBridge>,
    pub risk: Arc<RiskEngine>,
    pub broadcast_tx: broadcast::Sender<ServerMessage>,
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
    let risk = Arc::new(RiskEngine::new());

    let state = AppState {
        config: config.clone(),
        orderbook: orderbook.clone(),
        perps: perps.clone(),
        polymarket: polymarket.clone(),
        risk: risk.clone(),
        broadcast_tx: broadcast_tx.clone(),
    };

    // Spawn Hyperliquid feed
    let tx_clone = broadcast_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = hyperliquid::start_hyperliquid_feed(tx_clone).await {
            error!("Hyperliquid WebSocket feed error: {e}");
        }
    });

    let tx_clone = broadcast_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = hyperliquid::start_hyperliquid_stats_poller(tx_clone).await {
            error!("Hyperliquid stats poller error: {e}");
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

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    ws::handle_client(socket, state).await;
}
