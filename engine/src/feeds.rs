use crate::ws::ServerMessage;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, info};
use crate::polymarket::PolymarketBridge;
use crate::jupiter_prediction::JupiterPredictionBridge;

/// Starts the real-time Polymarket opportunity feed
pub async fn start_polymarket_feed(bridge: Arc<PolymarketBridge>, tx: broadcast::Sender<ServerMessage>) -> Result<()> {
    info!("⬡ Polymarket real-time opportunity feed starting...");
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
    let mut backoff = tokio::time::Duration::from_secs(2);

    loop {
        interval.tick().await;
        
        match bridge.discover_opportunities().await {
            Ok(opps) => {
                let _ = tx.send(ServerMessage::PolymarketOpportunities { opportunities: opps });
                backoff = tokio::time::Duration::from_secs(2); // reset on success
            }
            Err(e) => {
                error!("Polymarket opportunity discovery error: {}. Backing off...", e);
                tokio::time::sleep(backoff).await;
                backoff = std::cmp::min(backoff * 2, tokio::time::Duration::from_secs(60));
            }
        }
    }
}

/// Starts the real-time Jupiter Prediction opportunity feed
pub async fn start_jupiter_prediction_feed(bridge: Arc<JupiterPredictionBridge>, tx: broadcast::Sender<ServerMessage>) -> Result<()> {
    info!("⬡ Jupiter Prediction real-time opportunity feed starting...");
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
    let mut backoff = tokio::time::Duration::from_secs(2);

    loop {
        interval.tick().await;
        
        match bridge.discover_opportunities().await {
            Ok(opps) => {
                let _ = tx.send(ServerMessage::JupiterPredictionOpportunities { opportunities: opps });
                backoff = tokio::time::Duration::from_secs(2); // reset on success
            }
            Err(e) => {
                error!("Jupiter Prediction opportunity discovery error: {}. Backing off...", e);
                tokio::time::sleep(backoff).await;
                backoff = std::cmp::min(backoff * 2, tokio::time::Duration::from_secs(60));
            }
        }
    }
}
