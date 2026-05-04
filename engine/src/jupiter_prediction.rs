use crate::{config::EngineConfig, ws::ServerMessage};
use anyhow::{Context, Result};
use futures_util::future::join_all;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn, error};

// ─── Jupiter Prediction API Types ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterPredictionEvent {
    #[serde(rename = "eventId")]
    pub event_id: String,
    #[serde(rename = "isActive")]
    pub is_active: bool,
    #[serde(rename = "isLive")]
    pub is_live: bool,
    pub category: String,
    pub subcategory: Option<String>,
    pub tags: Vec<String>,
    pub metadata: JupiterPredictionMetadata,
    pub markets: Vec<JupiterPredictionMarket>,
    #[serde(rename = "volumeUsd")]
    pub volume_usd: String, // API returns as string due to precision
    #[serde(rename = "volume24hr")]
    pub volume_24h: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterPredictionMetadata {
    pub title: String,
    pub slug: String,
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
    pub series: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterPredictionMarket {
    #[serde(rename = "marketId")]
    pub market_id: String,
    pub status: String,
    pub title: String,
    pub pricing: JupiterPredictionPricing,
    pub outcomes: Vec<String>,
    #[serde(rename = "clobTokenIds")]
    pub clob_token_ids: Vec<String>,
    #[serde(rename = "outcomePrices")]
    pub outcome_prices: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterPredictionPricing {
    #[serde(rename = "buyYesPriceUsd")]
    pub buy_yes_price_usd: u64,
    #[serde(rename = "sellYesPriceUsd")]
    pub sell_yes_price_usd: u64,
    #[serde(rename = "sellNoPriceUsd")]
    pub sell_no_price_usd: u64,
    #[serde(rename = "buyNoPriceUsd")]
    pub buy_no_price_usd: u64,
    pub volume: u64,
}

// ─── Soldex Internal Representation ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterOpportunity {
    pub event_id: String,
    pub market_id: String,
    pub title: String,
    pub image_url: Option<String>,
    pub outcomes: Vec<String>,
    pub outcome_prices: Vec<f64>,
    pub clob_token_ids: Vec<String>,
    pub volume_24h: f64,
    pub volume_usd: f64,
    pub category: String,
    pub is_live: bool,
    pub source: String, // "jupiter"
}

// ─── Bridge ──────────────────────────────────────────────────────────────────

pub struct JupiterPredictionBridge {
    config: Arc<EngineConfig>,
    client: reqwest::Client,
    pub opportunities: Arc<dashmap::DashMap<String, JupiterOpportunity>>,
}

impl JupiterPredictionBridge {
    pub fn new(config: Arc<EngineConfig>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("soldex-engine/1.0")
            .build()
            .expect("HTTP client");
        Self { 
            config, 
            client,
            opportunities: Arc::new(dashmap::DashMap::new()),
        }
    }

    pub async fn fetch_events_for_category(
        &self, 
        category: &str, 
        filter: Option<&str>
    ) -> Result<Vec<JupiterPredictionEvent>> {
        let url = format!("{}/api/v1/events", self.config.jupiter_prediction_api_url);
        
        let mut query = vec![
            ("includeMarkets", "true"),
            ("start", "0"),
            ("end", "12"),
            ("sortBy", "volume24hr"),
            ("sortDirection", "desc"),
            ("category", category),
        ];

        if let Some(f) = filter {
            query.push(("filter", f));
        }

        let resp = self
            .client
            .get(&url)
            .query(&query)
            .send()
            .await
            .context(format!("GET /api/v1/events?category={}", category))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Jupiter API error {} for category {}: {}", status, category, body);
        }

        #[derive(Deserialize)]
        struct Wrapper {
            data: Vec<JupiterPredictionEvent>,
        }

        let wrapper: Wrapper = resp.json().await.context("Parsing Jupiter response")?;
        Ok(wrapper.data)
    }

    pub async fn discover_opportunities(&self) -> Result<Vec<JupiterOpportunity>> {
        // Define categories as per the reference implementation
        let categories = vec![
            ("all", Some("live")),
            ("sports", None),
            ("crypto", None),
            ("politics", None),
            ("esports", None),
            ("culture", None),
            ("economics", None),
            ("tech", None),
            ("finance", None),
            ("weather", None),
            ("mentions", None),
        ];

        // Launch all fetches in parallel
        let futures: Vec<_> = categories.iter().map(|(cat, filter)| {
            self.fetch_events_for_category(cat, *filter)
        }).collect();

        let results = join_all(futures).await;
        
        // Deduplicate by market_id since an event might span categories or be in "all"
        let mut opp_map: HashMap<String, JupiterOpportunity> = HashMap::new();

        for (i, result) in results.into_iter().enumerate() {
            let cat_name = categories[i].0;
            match result {
                Ok(events) => {
                    for event in events {
                        if !event.is_active {
                            continue;
                        }

                        for market in event.markets {
                            if market.status != "open" {
                                continue;
                            }

                            let outcome_prices: Vec<f64> = market
                                .outcome_prices
                                .iter()
                                .map(|p| p.parse::<f64>().unwrap_or(0.0))
                                .collect();

                            // API returns volumes in micro-USD (6 decimals)
                            let volume_24h = event.volume_24h.parse::<f64>().unwrap_or(0.0) / 1_000_000.0;
                            let volume_usd = event.volume_usd.parse::<f64>().unwrap_or(0.0) / 1_000_000.0;

                            let opp = JupiterOpportunity {
                                event_id: event.event_id.clone(),
                                market_id: market.market_id.clone(),
                                title: format!("{}: {}", event.metadata.title, market.title),
                                image_url: event.metadata.image_url.clone(),
                                outcomes: market.outcomes.clone(),
                                outcome_prices,
                                clob_token_ids: market.clob_token_ids.clone(),
                                volume_24h,
                                volume_usd,
                                category: event.category.clone(),
                                is_live: event.is_live,
                                source: "jupiter".into(),
                            };

                            // Only insert if not exists or higher volume (shouldn't happen with same ID)
                            opp_map.insert(opp.market_id.clone(), opp.clone());
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to fetch Jupiter category '{}': {}", cat_name, e);
                }
            }
        }

        // Update cache
        for opp in opp_map.values() {
            self.opportunities.insert(opp.market_id.clone(), opp.clone());
        }

        let mut all_opps: Vec<JupiterOpportunity> = opp_map.into_values().collect();
        
        // Sort by volume descending for the best discovery experience
        all_opps.sort_by(|a, b| b.volume_24h.partial_cmp(&a.volume_24h).unwrap_or(std::cmp::Ordering::Equal));

        info!("Aggregated {} unique Jupiter opportunities across categories", all_opps.len());
        Ok(all_opps)
    }

    pub async fn execute_order(
        &self,
        market_id: &str,
        outcome_index: usize,
        size_usdc: f64,
        owner_pubkey: &str,
    ) -> Result<Vec<u8>> {
        // 1. Get market from cache
        let opp = self.opportunities.get(market_id)
            .context("Market not found in cache")?
            .clone();

        let token_id = opp.clob_token_ids.get(outcome_index)
            .context("Outcome index out of bounds")?;

        // 2. Get quote from Jupiter
        // USDC mint: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
        let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
        let amount_atomic = (size_usdc * 1_000_000.0) as u64; // USDC has 6 decimals

        let quote_url = "https://quote-api.jup.ag/v6/quote";
        let quote_resp = self.client.get(quote_url)
            .query(&[
                ("inputMint", usdc_mint),
                ("outputMint", token_id.as_str()),
                ("amount", &amount_atomic.to_string()),
                ("slippageBps", "50"),
            ])
            .send()
            .await
            .context("Jupiter quote request failed")?;

        if !quote_resp.status().is_success() {
            let err = quote_resp.text().await.unwrap_or_default();
            anyhow::bail!("Jupiter quote error: {}", err);
        }

        let quote_json: serde_json::Value = quote_resp.json().await?;

        // 3. Get swap transaction
        let swap_url = "https://quote-api.jup.ag/v6/swap";
        let swap_resp = self.client.post(swap_url)
            .json(&serde_json::json!({
                "quoteResponse": quote_json,
                "userPublicKey": owner_pubkey,
                "wrapAndUnwrapSol": true
            }))
            .send()
            .await
            .context("Jupiter swap request failed")?;

        if !swap_resp.status().is_success() {
            let err = swap_resp.text().await.unwrap_or_default();
            anyhow::bail!("Jupiter swap error: {}", err);
        }

        #[derive(Deserialize)]
        struct SwapResponse {
            #[serde(rename = "swapTransaction")]
            swap_transaction: String,
        }

        let swap_data: SwapResponse = swap_resp.json().await?;
        
        // 4. Decode base64 transaction
        use base64::{Engine as _, engine::general_purpose};
        let tx_bytes = general_purpose::STANDARD.decode(swap_data.swap_transaction)
            .context("Failed to decode jupiter swap transaction")?;

        Ok(tx_bytes)
    }
}
