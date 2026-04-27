use crate::perps::MarketTicker;
use crate::ws::ServerMessage;
use anyhow::{Result, Context};
use serde::Deserialize;
use std::collections::HashMap;
use tokio::sync::broadcast;
use tokio::time::{self, Duration};
use tracing::{error, info, debug};
use futures_util::StreamExt;
use reqwest_eventsource::{Event, EventSource};

const SOL_USDC: &str = "SOL-USDC";
const BTC_USDC: &str = "BTC-USDC";
const ETH_USDC: &str = "ETH-USDC";
const JUP_USDC: &str = "JUP-USDC";

const PYTH_SOL: &str = "ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d";
const PYTH_BTC: &str = "e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43";
const PYTH_ETH: &str = "ff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace";
const PYTH_JUP: &str = "0a0408d619e9380abad35060f9192039ed5042fa6f82301d0e48bb52be830996";

pub fn pyth_id_to_market(id: &str) -> Option<&'static str> {
    match id {
        PYTH_SOL => Some(SOL_USDC),
        PYTH_BTC => Some(BTC_USDC),
        PYTH_ETH => Some(ETH_USDC),
        PYTH_JUP => Some(JUP_USDC),
        _ => None,
    }
}

pub fn mint_to_market(mint: &str) -> Option<&'static str> {
    match mint {
        "So11111111111111111111111111111111111111112" => Some(SOL_USDC),
        "3NZ9J7PruDUoGI34D6Nj4LpsJoiqcMWCADJBR1GjF8sk" => Some(BTC_USDC),
        "7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs" => Some(ETH_USDC),
        "JUPyiwrYJFskR4AnmabSHv4m6v9U2Y8KPhZndK1yMNo" => Some(JUP_USDC),
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
struct PythPriceInfo {
    pub price: String,
    pub expo: i32,
}

#[derive(Debug, Deserialize)]
struct PythParsedUpdate {
    pub id: String,
    pub price: PythPriceInfo,
}

#[derive(Debug, Deserialize)]
struct PythMessage {
    pub parsed: Option<Vec<PythParsedUpdate>>,
}

pub async fn start_jupiter_feed(tx: broadcast::Sender<ServerMessage>) -> Result<()> {
    info!("🪐 Jupiter Perps real-time feed starting (Pyth Hermes SSE)...");

    let ids = vec![PYTH_SOL, PYTH_BTC, PYTH_ETH, PYTH_JUP];
    let query_params = ids.iter().map(|id| format!("ids[]={}", id)).collect::<Vec<_>>().join("&");
    let sse_url = format!("https://hermes.pyth.network/v2/updates/price/stream?{}&parsed=true", query_params);

    loop {
        let mut es = EventSource::get(&sse_url);
        info!("✅ Connected to Pyth Hermes SSE stream");

        while let Some(event) = es.next().await {
            match event {
                Ok(Event::Message(message)) => {
                    if let Ok(pyth_msg) = serde_json::from_str::<PythMessage>(&message.data) {
                        if let Some(parsed_updates) = pyth_msg.parsed {
                            for p in parsed_updates {
                                if let Some(market_id) = pyth_id_to_market(&p.id) {
                                    let raw_price = p.price.price.parse::<f64>().unwrap_or(0.0);
                                    let expo = p.price.expo;
                                    let price = raw_price * 10f64.powi(expo);

                                    info!("🚀 Real-time Price: {} = ${:.4}", market_id, price);
                                    broadcast_ticker_update(market_id, price, &tx);
                                }
                            }
                        }
                    } else {
                        // Log raw if deserialization fails to debug further if needed
                        debug!("Failed to deserialize Pyth message: {}", message.data);
                    }
                }
                Err(e) => {
                    error!("Pyth SSE error: {}. Reconnecting in 5s...", e);
                    es.close();
                    break;
                }
                _ => {}
            }
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

fn broadcast_ticker_update(market_id: &'static str, price: f64, tx: &broadcast::Sender<ServerMessage>) {
    let ticker = MarketTicker {
        market_id: market_id.to_string(),
        price,
        price_24h_ago: 0.0,
        change_24h: 0.0,
        change_pct_24h: 0.0,
        volume_24h: 0.0,
        high_24h: 0.0,
        low_24h: 0.0,
        open_interest: 0.0,
        funding_rate: 0.0,
        next_funding_ts: 0,
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
    };
    let _ = tx.send(ServerMessage::TickerUpdate { ticker });

    // Also broadcast synthetic orderbook
    use crate::orderbook::{OrderbookSnapshot, PriceLevel};
    use rust_decimal::Decimal;
    use rust_decimal::prelude::FromPrimitive;

    let snapshot = OrderbookSnapshot {
        market_id: market_id.to_string(),
        bids: (1..5).map(|i| PriceLevel {
            price: Decimal::from_f64(price - (i as f64 * 0.01)).unwrap_or_default(),
            total_qty: Decimal::from_f64(100.0 + (i as f64 * 50.0)).unwrap_or_default(),
            order_count: 5,
        }).collect(),
        asks: (1..5).map(|i| PriceLevel {
            price: Decimal::from_f64(price + (i as f64 * 0.01)).unwrap_or_default(),
            total_qty: Decimal::from_f64(100.0 + (i as f64 * 50.0)).unwrap_or_default(),
            order_count: 5,
        }).collect(),
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
    };
    let _ = tx.send(ServerMessage::Snapshot(snapshot));
}

#[derive(Debug, Deserialize)]
struct JupMarketStats {
    pub price: String,
    #[serde(rename = "priceChange24H")]
    pub price_change_24h: String,
    #[serde(rename = "priceHigh24H")]
    pub high_24h: String,
    #[serde(rename = "priceLow24H")]
    pub low_24h: String,
    pub volume: String,
}

pub async fn start_jupiter_stats_poller(tx: broadcast::Sender<ServerMessage>) -> Result<()> {
    info!("📊 Jupiter Perps stats poller starting...");
    
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let mut interval = time::interval(Duration::from_secs(30));
    
    let mints = vec![
        "So11111111111111111111111111111111111111112",
        "3NZ9J7PruDUoGI34D6Nj4LpsJoiqcMWCADJBR1GjF8sk",
        "7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs",
        "JUPyiwrYJFskR4AnmabSHv4m6v9U2Y8KPhZndK1yMNo",
    ];

    loop {
        interval.tick().await;

        for mint in &mints {
            let url = format!("https://perps-api.jup.ag/v1/market-stats?mint={}", mint);
            let res = client.get(&url).send().await;
            
            if let Ok(resp) = res {
                if resp.status().is_success() {
                    if let Ok(stats) = resp.json::<JupMarketStats>().await {
                        let price = stats.price.parse::<f64>().unwrap_or(0.0);
                        let change = stats.price_change_24h.parse::<f64>().unwrap_or(0.0);
                        let high = stats.high_24h.parse::<f64>().unwrap_or(0.0);
                        let low = stats.low_24h.parse::<f64>().unwrap_or(0.0);
                        let vol = stats.volume.parse::<f64>().unwrap_or(0.0);

                        if let Some(market_id) = mint_to_market(mint) {
                            let ticker = MarketTicker {
                                market_id: market_id.to_string(),
                                price,
                                price_24h_ago: price / (1.0 + (change / 100.0)),
                                change_24h: price * (change / 100.0),
                                change_pct_24h: change,
                                volume_24h: vol,
                                high_24h: high,
                                low_24h: low,
                                open_interest: 0.0,
                                funding_rate: 0.0,
                                next_funding_ts: 0,
                                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
                            };
                            let _ = tx.send(ServerMessage::TickerUpdate { ticker });
                        }
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
}
