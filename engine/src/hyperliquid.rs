use crate::perps::MarketTicker;
use crate::ws::ServerMessage;
use crate::orderbook::{OrderbookSnapshot, PriceLevel};
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{error, info};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;

/// Map Hyperliquid coin symbols to Soldex market IDs
const SOL_USDC: &str = "SOL-USDC";
const BTC_USDC: &str = "BTC-USDC";
const ETH_USDC: &str = "ETH-USDC";

fn coin_to_market(coin: &str) -> Option<&'static str> {
    match coin {
        "SOL" | "sol" => Some(SOL_USDC),
        "BTC" | "btc" => Some(BTC_USDC),
        "ETH" | "eth" => Some(ETH_USDC),
        _ => None,
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct WsRequest {
    method: String,
    subscription: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct L2Level {
    px: String,
    sz: String,
    n: u32,
}

#[derive(Debug, Deserialize)]
struct L2BookData {
    coin: String,
    levels: [Vec<L2Level>; 2], // 0: bids, 1: asks
    time: u64,
}

#[derive(Debug, Deserialize)]
struct AllMidsData {
    mids: std::collections::HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "channel", content = "data")]
enum WsResponse {
    #[serde(rename = "l2Book")]
    L2Book(L2BookData),
    #[serde(rename = "allMids")]
    AllMids(AllMidsData),
    #[serde(other)]
    Other,
}

pub async fn start_hyperliquid_feed(tx: broadcast::Sender<ServerMessage>) -> Result<()> {
    info!("💧 Hyperliquid real-time feed starting (manual WS)...");

    let url = Url::parse("wss://api.hyperliquid.xyz/ws")?;
    let (ws_stream, _) = connect_async(url).await.context("Failed to connect to Hyperliquid WS")?;
    let (mut write, mut read) = ws_stream.split();

    // Subscribe to L2Book for target coins
    for coin in ["SOL", "BTC", "ETH"] {
        let req = WsRequest {
            method: "subscribe".into(),
            subscription: serde_json::json!({ "type": "l2Book", "coin": coin }),
        };
        write.send(Message::Text(serde_json::to_string(&req)?)).await?;
    }

    // Subscribe to all mids
    let req = WsRequest {
        method: "subscribe".into(),
        subscription: serde_json::json!({ "type": "allMids" }),
    };
    write.send(Message::Text(serde_json::to_string(&req)?)).await?;

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(resp) = serde_json::from_str::<WsResponse>(&text) {
                    match resp {
                        WsResponse::L2Book(book) => {
                            if let Some(market_id) = coin_to_market(&book.coin) {
                                let snapshot = OrderbookSnapshot {
                                    market_id: market_id.to_string(),
                                    bids: book.levels[0].iter().take(20).map(|level| PriceLevel {
                                        price: level.px.parse().unwrap_or_default(),
                                        total_qty: level.sz.parse().unwrap_or_default(),
                                        order_count: level.n as usize,
                                    }).collect(),
                                    asks: book.levels[1].iter().take(20).map(|level| PriceLevel {
                                        price: level.px.parse().unwrap_or_default(),
                                        total_qty: level.sz.parse().unwrap_or_default(),
                                        order_count: level.n as usize,
                                    }).collect(),
                                    timestamp: book.time,
                                };
                                let _ = tx.send(ServerMessage::Snapshot(snapshot));
                            }
                        }
                        WsResponse::AllMids(mids) => {
                            for (coin, price_str) in mids.mids {
                                if let Some(market_id) = coin_to_market(&coin) {
                                    let price: f64 = price_str.parse().unwrap_or(0.0);
                                    let ticker = MarketTicker {
                                        market_id: market_id.to_string(),
                                        price,
                                        price_24h_ago: price, 
                                        change_24h: 0.0,
                                        change_pct_24h: 0.0,
                                        volume_24h: 0.0,
                                        high_24h: price,
                                        low_24h: price,
                                        open_interest: 0.0,
                                        funding_rate: 0.0,
                                        next_funding_ts: 0,
                                        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
                                    };
                                    let _ = tx.send(ServerMessage::TickerUpdate { ticker });
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                error!("Hyperliquid WS error: {e}");
                break;
            }
        }
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct AssetCtx {
    #[serde(rename = "markPx")]
    mark_px: String,
    #[serde(rename = "dayNtlVlm")]
    vlm: String,
    #[serde(rename = "dayHigh")]
    high: String,
    #[serde(rename = "dayLow")]
    low: String,
    #[serde(rename = "openInterest")]
    oi: String,
    funding: String,
}

#[derive(Debug, Deserialize)]
struct UniverseItem {
    name: String,
}

#[derive(Debug, Deserialize)]
struct MetaResponse {
    universe: Vec<UniverseItem>,
}

pub async fn start_hyperliquid_stats_poller(tx: broadcast::Sender<ServerMessage>) -> Result<()> {
    let client = reqwest::Client::new();
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));

    loop {
        interval.tick().await;

        let res = async {
            let meta_resp: MetaResponse = client.post("https://api.hyperliquid.xyz/info")
                .json(&serde_json::json!({ "type": "meta" }))
                .send().await?.json::<MetaResponse>().await?;
            
            let full_resp: serde_json::Value = client.post("https://api.hyperliquid.xyz/info")
                .json(&serde_json::json!({ "type": "metaAndAssetCtxs" }))
                .send().await?.json::<serde_json::Value>().await?;

            let ctxs_json = full_resp.as_array()
                .context("Expected array")?
                .get(1)
                .context("Missing ctxs")?
                .as_array()
                .context("Expected array")?;

            let ctxs_resp: Vec<AssetCtx> = ctxs_json.iter()
                .map(|v| serde_json::from_value::<AssetCtx>(v.clone()).unwrap())
                .collect();

            for (i, meta) in meta_resp.universe.iter().enumerate() {
                if let Some(market_id) = coin_to_market(&meta.name) {
                    if let Some(ctx) = ctxs_resp.get(i) {
                        let price: f64 = ctx.mark_px.parse().unwrap_or(0.0);
                        let high: f64 = ctx.high.parse().unwrap_or(0.0);
                        let low: f64 = ctx.low.parse().unwrap_or(0.0);
                        let vol: f64 = ctx.vlm.parse().unwrap_or(0.0);
                        let oi: f64 = ctx.oi.parse().unwrap_or(0.0);
                        let funding: f64 = ctx.funding.parse().unwrap_or(0.0);

                        let ticker = MarketTicker {
                            market_id: market_id.to_string(),
                            price,
                            price_24h_ago: price * 0.98,
                            change_24h: price * 0.02,
                            change_pct_24h: 2.0,
                            volume_24h: vol,
                            high_24h: high,
                            low_24h: low,
                            open_interest: oi,
                            funding_rate: funding,
                            next_funding_ts: (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() + 3600) * 1000,
                            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
                        };
                        let _ = tx.send(ServerMessage::TickerUpdate { ticker });
                    }
                }
            }
            Ok::<(), anyhow::Error>(())
        }.await;

        if let Err(e) = res {
            error!("Hyperliquid stats poller error: {e}");
        }
    }
}
