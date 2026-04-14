use anyhow::Result;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
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
mod polymarket;
mod risk;
mod wallet;
mod ws;

use config::EngineConfig;
use orderbook::OrderbookEngine;
use perps::PerpsEngine;
use polymarket::PolymarketBridge;
use risk::RiskEngine;
use ws::{ClientMessage, ServerMessage};

/// Shared engine state across all WebSocket connections
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<EngineConfig>,
    pub orderbook: Arc<OrderbookEngine>,
    pub perps: Arc<PerpsEngine>,
    pub polymarket: Arc<PolymarketBridge>,
    pub risk: Arc<RiskEngine>,
    /// Broadcast channel — engine pushes updates to ALL connected clients
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
    info!("🔗 Solana RPC: {}", config.solana_rpc_url);
    info!("📊 Polymarket API: {}", config.polymarket_api_url);

    // Broadcast channel for pushing market data to frontend clients
    let (broadcast_tx, _) = broadcast::channel::<ServerMessage>(1024);

    // Initialize sub-engines
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

    // Spawn background tasks
    let pm_bridge = polymarket.clone();
    let tx_clone = broadcast_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = feeds::start_polymarket_feed(pm_bridge, tx_clone).await {
            error!("Polymarket feed error: {e}");
        }
    });

    let tx_clone = broadcast_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = feeds::start_pyth_feed(tx_clone).await {
            error!("Pyth market data error: {e}");
        }
    });

    // Build Axum router
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/health", get(health_handler))
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&config.listen_addr).await?;
    info!("✅ Engine listening on {}", config.listen_addr);

    axum::serve(listener, app).await?;
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
