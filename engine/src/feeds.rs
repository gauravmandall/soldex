use crate::ws::ServerMessage;
use anyhow::{Context, Result};
use futures_util::StreamExt;
use reqwest_eventsource::{Event, EventSource};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, info, warn};
use crate::perps::MarketTicker;

// Pyth Hermes SSE URL
const PYTH_HERMES_URL: &str = "https://hermes.pyth.network/v2/updates/price/stream";

// Feed IDs for SOL/USD, BTC/USD, ETH/USD on Pyth
const SOL_USD_FEED: &str = "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d";
const BTC_USD_FEED: &str = "0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43";
const ETH_USD_FEED: &str = "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace";

#[derive(Debug, Deserialize)]
struct PythUpdate {
    binary: PythBinary,
    parsed: Vec<PythParsedPrice>,
}

#[derive(Debug, Deserialize)]
struct PythBinary {
    encoding: String,
    data: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PythParsedPrice {
    id: String,
    price: PythPriceData,
    ema_price: PythPriceData,
    metadata: PythMetadata,
}

#[derive(Debug, Deserialize)]
struct PythPriceData {
    price: String,
    conf: String,
    expo: i32,
    publish_time: u64,
}

#[derive(Debug, Deserialize)]
struct PythMetadata {
    slot: u64,
    proof_available_time: u64,
    prev_publish_time: u64,
}

use crate::polymarket::PolymarketBridge;

/// Starts the real-time Pyth price feed via SSE
pub async fn start_pyth_feed(tx: broadcast::Sender<ServerMessage>) -> Result<()> {
    // ... (logic remains same, just ensuring correct imports and context)
    info!("🐍 Pyth real-time feed starting...");
    
    let url = format!(
        "{}?parsed=true&ids[]={}&ids[]={}&ids[]={}",
        PYTH_HERMES_URL, SOL_USD_FEED, BTC_USD_FEED, ETH_USD_FEED
    );

    let mut es = EventSource::get(&url);

    while let Some(event) = es.next().await {
        match event {
            Ok(Event::Message(message)) => {
                if let Ok(update) = serde_json::from_str::<PythUpdate>(&message.data) {
                    for price in update.parsed {
                        let price_val = price.price.price.parse::<f64>().unwrap_or(0.0) 
                                        * 10f64.powi(price.price.expo);
                        
                        let market_id = match price.id.as_str() {
                            SOL_USD_FEED => "SOL-PERP",
                            BTC_USD_FEED => "BTC-PERP",
                            ETH_USD_FEED => "ETH-PERP",
                            _ => continue,
                        };

                        let ticker = MarketTicker {
                            market_id: market_id.to_string(),
                            price: price_val,
                            price_24h_ago: price_val * 0.98,
                            change_24h: price_val * 0.02,
                            change_pct_24h: 2.0,
                            volume_24h: 1_000_000.0,
                            high_24h: price_val * 1.01,
                            low_24h: price_val * 0.99,
                            open_interest: 50_000_000.0,
                            funding_rate: 0.0001,
                            next_funding_ts: (std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs() + 3600) * 1000,
                            timestamp: price.price.publish_time * 1000,
                        };

                        let _ = tx.send(ServerMessage::TickerUpdate { ticker });
                    }
                }
            }
            Ok(Event::Open) => info!("Connected to Pyth Hermes SSE"),
            Err(e) => {
                error!("Pyth SSE error: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        }
    }

    Ok(())
}

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
