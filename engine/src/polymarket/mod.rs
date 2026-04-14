/// Polymarket CLOB Bridge
/// Discovers active markets on Polymarket and executes orders
/// via the CLOB (Central Limit Order Book) API.

use crate::{config::EngineConfig, ws::ServerMessage};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::{sync::broadcast, time};
use tracing::{debug, error, info, warn};

const POLL_INTERVAL_MS: u64 = 5_000;

// ─── Polymarket REST types ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolymarketToken {
    pub token_id: String,
    pub outcome: String, // "Yes" | "No"
    pub price: f64,
    pub winner: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolymarketMarket {
    pub condition_id: String,
    pub question_id: String,
    pub question: String,
    pub description: Option<String>,
    pub end_date_iso: Option<String>,
    pub game_start_time: Option<String>,
    pub tokens: Vec<PolymarketToken>,
    pub active: bool,
    pub closed: bool,
    pub volume: Option<f64>,
    pub volume_num_24hr: Option<f64>,
    pub liquidity: Option<f64>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolymarketOrderbook {
    pub market: Option<String>, 
    pub asset_id: Option<String>,
    pub bids: Option<Vec<PolyPriceLevel>>,
    pub asks: Option<Vec<PolyPriceLevel>>,
    pub hash: Option<String>,
    pub timestamp: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolyPriceLevel {
    pub price: String,
    pub size: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostOrderRequest {
    pub order_type: String, // "FOK" | "GTC" | "GTD"
    pub token_id: String,
    pub side: String,   // "BUY" | "SELL"
    pub price: f64,
    pub size: f64,
    pub expiration: Option<u64>,
    pub nonce: u64,
    pub fee_rate_bps: u32,
    pub signature: String, // L1 EIP-712 signature
    pub signer: String,    // Ethereum address (Polymarket uses Polygon)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostOrderResponse {
    pub order_id: String,
    pub status: String,
    pub success: bool,
}

// ─── Soldex internal representation ─────────────────────────────────────────

/// A Polymarket opportunity surfaced to Soldex traders
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolyOpportunity {
    pub market_id: String,       // condition_id
    pub question: String,
    pub category: String,
    pub end_date: String,
    pub yes_price: f64,          // 0.0–1.0 probability
    pub no_price: f64,
    pub volume_24h: f64,
    pub liquidity: f64,
    pub best_bid: f64,
    pub best_ask: f64,
    pub spread: f64,
    pub source: String,          // "polymarket"
}

// ─── Bridge ──────────────────────────────────────────────────────────────────

pub struct PolymarketBridge {
    config: Arc<EngineConfig>,
    client: reqwest::Client,
}

impl PolymarketBridge {
    pub fn new(config: Arc<EngineConfig>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("soldex-engine/0.1")
            .build()
            .expect("HTTP client");
        Self { config, client }
    }

    /// Fetch all active markets from Polymarket CLOB API
    pub async fn fetch_active_markets(&self) -> Result<Vec<PolymarketMarket>> {
        let url = format!("{}/markets", self.config.polymarket_api_url);
        let resp = self
            .client
            .get(&url)
            .query(&[
                ("active", "true"),
                ("closed", "false"),
                ("limit", "50"),
                ("order", "volume_num_24hr"),
                ("ascending", "false"),
            ])
            .send()
            .await
            .context("GET /markets")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Polymarket API error {}: {}", status, body);
        }

        #[derive(Deserialize)]
        struct Wrapper {
            data: Vec<PolymarketMarket>,
        }

        let wrapper: Wrapper = resp.json().await?;
        Ok(wrapper.data)
    }

    /// Fetch orderbook for a specific token (YES or NO side)
    pub async fn fetch_orderbook(&self, token_id: &str) -> Result<PolymarketOrderbook> {
        let url = format!("{}/book", self.config.polymarket_api_url);
        let resp = self
            .client
            .get(&url)
            .query(&[("token_id", token_id)])
            .send()
            .await
            .context("GET /book")?;

        let body = resp.text().await.context("read response text")?;
        
        // Try parsing into our struct which handles the 'error' field too
        let book: PolymarketOrderbook = serde_json::from_str(&body)
            .with_context(|| format!("Failed to decode orderbook JSON: {}", body))?;
            
        Ok(book)
    }

    /// Convert raw markets to Soldex opportunities with orderbook data
    pub async fn discover_opportunities(&self) -> Result<Vec<PolyOpportunity>> {
        let markets = self.fetch_active_markets().await?;
        let mut opps = Vec::new();

        for market in markets.iter().take(20) {
            // Find YES token
            let yes_token = market.tokens.iter().find(|t| t.outcome == "Yes");
            let no_token = market.tokens.iter().find(|t| t.outcome == "No");

            let (yes_price, no_price) = match (yes_token, no_token) {
                (Some(y), Some(n)) => (y.price, n.price),
                _ => continue,
            };

            // Fetch orderbook for YES token to get real bid/ask
            let (best_bid, best_ask, spread) =
                if let Some(yes) = yes_token {
                    match self.fetch_orderbook(&yes.token_id).await {
                        Ok(book) if book.error.is_none() && book.bids.is_some() && book.asks.is_some() => {
                            let bids = book.bids.unwrap();
                            let asks = book.asks.unwrap();
                            let bid = bids.first().and_then(|b| b.price.parse::<f64>().ok()).unwrap_or(yes_price);
                            let ask = asks.first().and_then(|a| a.price.parse::<f64>().ok()).unwrap_or(yes_price);
                            (bid, ask, ask - bid)
                        }
                        Ok(book) => {
                            if let Some(err) = book.error {
                                debug!("No orderbook for {}: {}", market.condition_id, err);
                            }
                            (yes_price - 0.01, yes_price + 0.01, 0.02)
                        }
                        Err(e) => {
                            warn!("Orderbook fetch failed for {}: {e}", market.condition_id);
                            (yes_price - 0.01, yes_price + 0.01, 0.02)
                        }
                    }
                } else {
                    (yes_price - 0.01, yes_price + 0.01, 0.02)
                };

            opps.push(PolyOpportunity {
                market_id: market.condition_id.clone(),
                question: market.question.clone(),
                category: market.category.clone().unwrap_or_else(|| "Other".into()),
                end_date: market.end_date_iso.clone().unwrap_or_else(|| "N/A".into()),
                yes_price,
                no_price,
                volume_24h: market.volume_num_24hr.unwrap_or(0.0),
                liquidity: market.liquidity.unwrap_or(0.0),
                best_bid,
                best_ask,
                spread,
                source: "polymarket".into(),
            });
        }

        info!("Discovered {} Polymarket opportunities", opps.len());
        Ok(opps)
    }

    /// Execute an order on Polymarket CLOB
    /// NOTE: Polymarket uses Polygon + EIP-712 signatures.
    /// The engine must hold the user's delegated signing key or
    /// proxy the signature from the frontend.
    pub async fn execute_order(
        &self,
        token_id: &str,
        side: &str,
        price: f64,
        size: f64,
        signature: &str,
        signer: &str,
    ) -> Result<PostOrderResponse> {
        let url = format!("{}/order", self.config.polymarket_api_url);

        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let body = PostOrderRequest {
            order_type: "GTC".into(),
            token_id: token_id.to_string(),
            side: side.to_uppercase(),
            price,
            size,
            expiration: None,
            nonce,
            fee_rate_bps: 0,
            signature: signature.to_string(),
            signer: signer.to_string(),
        };

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("POST /order")?;

        Ok(resp.json().await?)
    }

}
