/// Perps Engine
/// Manages perpetual futures positions on Solana.
/// Submits transactions to the on-chain soldex-perps Anchor program
/// and mirrors state in-memory for low-latency reads.

use crate::{config::EngineConfig, ws::ServerMessage};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::{collections::HashMap, str::FromStr, sync::Arc};
use tokio::sync::{broadcast, RwLock};
use tracing::{error, info};

// ─── Market config ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerpsMarket {
    pub id: String,        // e.g. "SOL-PERP"
    pub base: String,      // "SOL"
    pub quote: String,     // "USDC"
    pub tick_size: f64,
    pub lot_size: f64,     // min qty
    pub max_leverage: f64,
    pub maker_fee: f64,    // 0.0002 = 0.02%
    pub taker_fee: f64,    // 0.0005
    pub funding_rate: f64, // 8h rate
    pub open_interest: f64,
    pub mark_price: f64,
    pub index_price: f64,
}

impl PerpsMarket {
    pub fn sol_perp() -> Self {
        Self {
            id: "SOL-USDC".into(),
            base: "SOL".into(),
            quote: "USDC".into(),
            tick_size: 0.01,
            lot_size: 0.1,
            max_leverage: 20.0,
            maker_fee: 0.0002,
            taker_fee: 0.0005,
            funding_rate: 0.0001,
            open_interest: 0.0,
            mark_price: 0.0,
            index_price: 0.0,
        }
    }

    pub fn btc_perp() -> Self {
        Self {
            id: "BTC-USDC".into(),
            base: "BTC".into(),
            quote: "USDC".into(),
            tick_size: 1.0,
            lot_size: 0.001,
            max_leverage: 20.0,
            maker_fee: 0.0002,
            taker_fee: 0.0005,
            funding_rate: 0.00012,
            open_interest: 0.0,
            mark_price: 0.0,
            index_price: 0.0,
        }
    }

    pub fn eth_perp() -> Self {
        Self {
            id: "ETH-USDC".into(),
            base: "ETH".into(),
            quote: "USDC".into(),
            tick_size: 0.1,
            lot_size: 0.01,
            max_leverage: 20.0,
            maker_fee: 0.0002,
            taker_fee: 0.0005,
            funding_rate: 0.00008,
            open_interest: 0.0,
            mark_price: 0.0,
            index_price: 0.0,
        }
    }
}

// ─── Position ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PositionSide {
    Long,
    Short,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub market_id: String,
    pub owner: String,          // wallet pubkey
    pub side: PositionSide,
    pub size: f64,              // in base asset
    pub entry_price: f64,
    pub mark_price: f64,
    pub collateral: f64,        // USDC
    pub leverage: f64,
    pub unrealized_pnl: f64,
    pub liquidation_price: f64,
    pub funding_payment: f64,
    pub opened_at: u64,
}

impl Position {
    pub fn compute_pnl(&mut self, mark_price: f64) {
        self.mark_price = mark_price;
        let pnl = match self.side {
            PositionSide::Long => (mark_price - self.entry_price) * self.size,
            PositionSide::Short => (self.entry_price - mark_price) * self.size,
        };
        self.unrealized_pnl = pnl;
    }

    pub fn compute_liquidation_price(&mut self) {
        let maintenance_margin = 0.05; // 5%
        let liq_price = match self.side {
            PositionSide::Long => {
                self.entry_price * (1.0 - 1.0 / self.leverage + maintenance_margin)
            }
            PositionSide::Short => {
                self.entry_price * (1.0 + 1.0 / self.leverage - maintenance_margin)
            }
        };
        self.liquidation_price = liq_price;
    }
}

// ─── Market data ticker ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketTicker {
    pub market_id: String,
    pub price: f64,
    pub price_24h_ago: f64,
    pub change_24h: f64,
    pub change_pct_24h: f64,
    pub volume_24h: f64,
    pub high_24h: f64,
    pub low_24h: f64,
    pub open_interest: f64,
    pub funding_rate: f64,
    pub next_funding_ts: u64,
    pub timestamp: u64,
}

// ─── Candle (OHLCV) ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub timestamp: u64, // unix seconds, start of bar
}

// ─── Engine ───────────────────────────────────────────────────────────────────

pub struct PerpsEngine {
    config: Arc<EngineConfig>,
    rpc: Arc<RpcClient>,
    pub markets: Arc<RwLock<HashMap<String, PerpsMarket>>>,
    pub positions: Arc<RwLock<HashMap<String, Vec<Position>>>>, // pubkey → positions
    pub tickers: Arc<RwLock<HashMap<String, MarketTicker>>>,
}

impl PerpsEngine {
    pub async fn new(config: Arc<EngineConfig>) -> Result<Self> {
        let rpc = Arc::new(RpcClient::new_with_commitment(
            config.solana_rpc_url.clone(),
            CommitmentConfig::confirmed(),
        ));

        let mut markets = HashMap::new();
        markets.insert("SOL-USDC".into(), PerpsMarket::sol_perp());
        markets.insert("BTC-USDC".into(), PerpsMarket::btc_perp());
        markets.insert("ETH-USDC".into(), PerpsMarket::eth_perp());

        Ok(Self {
            config,
            rpc,
            markets: Arc::new(RwLock::new(markets)),
            positions: Arc::new(RwLock::new(HashMap::new())),
            tickers: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Fetch price from Pyth or fallback to Jupiter price API
    pub async fn fetch_price(&self, symbol: &str) -> Result<f64> {
        // In production, use Pyth on-chain or Helius DAS
        // For dev: Jupiter price API
        let token_mint = match symbol {
            "SOL" => "So11111111111111111111111111111111111111112",
            "BTC" => "9n4nbM75f5Ui33ZbPYXn59EwSgE8CGsHtAeTH5YFeJ9E",
            "ETH" => "7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs",
            _ => return Ok(1.0),
        };

        let url = format!(
            "https://price.jup.ag/v6/price?ids={}",
            token_mint
        );

        let client = reqwest::Client::new();
        let resp: serde_json::Value = client.get(&url).send().await?.json().await?;

        let price = resp["data"][token_mint]["price"]
            .as_f64()
            .context("price not found")?;

        Ok(price)
    }

    /// Broadcast price feed to WebSocket subscribers
    pub async fn run_market_data_feed(&self, tx: broadcast::Sender<ServerMessage>) -> Result<()> {
        info!("📈 Perps market data feed starting");
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));

        loop {
            interval.tick().await;

            let mut tickers = self.tickers.write().await;
            let markets = self.markets.read().await;

            for (id, market) in markets.iter() {
                let symbol = &market.base;
                let price = self.fetch_price(symbol).await.unwrap_or_else(|_| {
                    // fallback to last known price
                    tickers.get(id).map(|t| t.price).unwrap_or(0.0)
                });

                let prev_price = tickers.get(id).map(|t| t.price).unwrap_or(price);
                let change = price - prev_price;

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;

                let ticker = MarketTicker {
                    market_id: id.clone(),
                    price,
                    price_24h_ago: price * 0.98, // placeholder — use real 24h in production
                    change_24h: price - price * 0.98,
                    change_pct_24h: 2.0,
                    volume_24h: 1_900_000.0,
                    high_24h: price * 1.015,
                    low_24h: price * 0.985,
                    open_interest: 45_000_000.0,
                    funding_rate: market.funding_rate,
                    next_funding_ts: now + 28_800_000, // +8h
                    timestamp: now,
                };

                tickers.insert(id.clone(), ticker.clone());

                let _ = tx.send(ServerMessage::TickerUpdate { ticker });
            }
        }
    }

    /// Open a perpetual position (submits Solana tx)
    /// The frontend signs the tx — this builds and returns the unsigned tx bytes.
    pub async fn build_open_position_tx(
        &self,
        owner: &str,
        market_id: &str,
        side: PositionSide,
        size: f64,
        leverage: f64,
        collateral_lamports: u64,
    ) -> Result<Vec<u8>> {
        let owner_key = Pubkey::from_str(owner)?;
        let program_id = Pubkey::from_str(&self.config.program_id)?;

        // Derive PDAs
        let market_seed = market_id.as_bytes();
        let (market_pda, _) = Pubkey::find_program_address(&[b"market", market_seed], &program_id);
        let (position_pda, _) = Pubkey::find_program_address(
            &[b"position", market_seed, owner_key.as_ref()],
            &program_id,
        );

        // In a real implementation, use Anchor client to build the IX
        // For now, return a placeholder serialized tx
        // anchor_client::Client::new_with_options(...)
        //   .program(program_id)
        //   .request()
        //   .accounts(accounts)
        //   .args(args)
        //   .instructions()

        info!(
            "Building open_position tx: {} {} {market_id} x{leverage}",
            if matches!(side, PositionSide::Long) { "LONG" } else { "SHORT" },
            size
        );

        // Placeholder: return empty bytes — replace with real IX
        Ok(vec![])
    }
}
