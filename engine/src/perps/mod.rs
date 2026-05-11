/// Perps Engine
/// Manages perpetual futures positions on Solana.
/// Submits transactions to the on-chain soldex-perps Anchor program
/// and mirrors state in-memory for low-latency reads.
use crate::{config::EngineConfig, ws::ServerMessage};
use anyhow::{Context, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::hash::Hash;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::collections::VecDeque;
use std::{collections::HashMap, str::FromStr, sync::Arc};
use tokio::sync::{broadcast, RwLock};
use tracing::{error, info};

// ─── Market config ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerpsMarket {
    pub id: String,    // e.g. "SOL-PERP"
    pub base: String,  // "SOL"
    pub quote: String, // "USDC"
    pub tick_size: f64,
    pub lot_size: f64, // min qty
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
    pub owner: String, // wallet pubkey
    pub side: PositionSide,
    pub size: f64, // in base asset
    pub entry_price: f64,
    pub mark_price: f64,
    pub collateral: f64, // USDC
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

#[derive(Clone)]
pub struct CachedMarket {
    pub market_pda: Pubkey,
    pub price_feed: Pubkey,
    pub quote_mint: Pubkey,
    pub vault: Pubkey,
}

// ─── Engine ───────────────────────────────────────────────────────────────────

pub struct PerpsEngine {
    config: Arc<EngineConfig>,
    rpc: Arc<RpcClient>,
    pub markets: Arc<RwLock<HashMap<String, PerpsMarket>>>,
    pub positions: Arc<RwLock<HashMap<String, Vec<Position>>>>, // pubkey → positions
    pub tickers: Arc<RwLock<HashMap<String, MarketTicker>>>,
    /// Per-market ring buffer: (unix_ms, price) — kept for 25h, sampled every 2s
    price_history: Arc<RwLock<HashMap<String, VecDeque<(u64, f64)>>>>,
    /// Cached on-chain OI + funding rate — refreshed every 30s to avoid RPC hammering
    market_stats_cache: Arc<RwLock<HashMap<String, (f64, f64)>>>, // feed_id → (open_interest_usd, funding_rate)
    pub market_cache: Arc<DashMap<[u8; 16], CachedMarket>>,
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
        markets.insert(
            "JUP-USDC".into(),
            PerpsMarket {
                id: "JUP-USDC".into(),
                base: "JUP".into(),
                quote: "USDC".into(),
                tick_size: 0.0001,
                lot_size: 1.0,
                max_leverage: 10.0,
                maker_fee: 0.0002,
                taker_fee: 0.0005,
                funding_rate: 0.00015,
                open_interest: 0.0,
                mark_price: 0.0,
                index_price: 0.0,
            },
        );

        Ok(Self {
            config,
            rpc,
            markets: Arc::new(RwLock::new(markets)),
            positions: Arc::new(RwLock::new(HashMap::new())),
            tickers: Arc::new(RwLock::new(HashMap::new())),
            price_history: Arc::new(RwLock::new(HashMap::new())),
            market_stats_cache: Arc::new(RwLock::new(HashMap::new())),
            market_cache: Arc::new(DashMap::new()),
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

        let url = format!("https://price.jup.ag/v6/price?ids={}", token_mint);

        let client = reqwest::Client::new();
        let resp: serde_json::Value = client.get(&url).send().await?.json().await?;

        let price = resp["data"][token_mint]["price"]
            .as_f64()
            .context("price not found")?;

        Ok(price)
    }

    /// Map feed market id ("SOL-USDC") → on-chain market id ("SOL-PERP")
pub fn feed_to_onchain(feed_id: &str) -> Option<&'static str> {        match feed_id {
            "SOL-USDC" => Some("SOL-PERP"),
            "BTC-USDC" => Some("BTC-PERP"),
            "ETH-USDC" => Some("ETH-PERP"),
            "JUP-USDC" => Some("JUP-PERP"),
            _ => None,
        }
    }

    /// Fetch open_interest + instantaneous funding_rate from on-chain MarketState.
    /// Uses same skew formula as funding_tick_er — no Pyth needed.
    async fn fetch_market_stats(&self, feed_id: &str, mark_price: f64) -> Option<(f64, f64)> {
        use anchor_lang::AccountDeserialize;
        use soldex_perps::state::MarketState;

        let onchain_id = Self::feed_to_onchain(feed_id)?;

        let mut market_id_16 = [0u8; 16];
        let b = onchain_id.as_bytes();
        market_id_16[..b.len().min(16)].copy_from_slice(&b[..b.len().min(16)]);

        let program_id = onchain_id
            .parse::<solana_sdk::pubkey::Pubkey>()
            .ok()
            .unwrap_or_else(|| self.config.program_id.parse().unwrap_or_default());
        let program_id: solana_sdk::pubkey::Pubkey = self.config.program_id.parse().ok()?;

        let (market_pda, _) = solana_sdk::pubkey::Pubkey::find_program_address(
            &[b"market", &market_id_16],
            &program_id,
        );

        let account = self.rpc.get_account(&market_pda).await.ok()?;
        let mut data = account.data.as_slice();
        let state = MarketState::try_deserialize(&mut data).ok()?;

        let long_oi = state.long_open_interest as f64;
        let short_oi = state.short_open_interest as f64;

        // open_interest in USD = (long_oi + short_oi) * mark_price / 1e6
        let open_interest_usd = (long_oi + short_oi) * mark_price / 1_000_000.0;

        // Instantaneous funding rate — same skew formula as funding_tick_er
        let total_notional = (long_oi + short_oi) * mark_price / 1_000_000.0;
        let skew_notional = (long_oi - short_oi) * mark_price / 1_000_000.0;

        const BASE_RATE: f64 = 0.0001; // 100_000_000 / 1e9
        let funding_rate = if total_notional.abs() < 0.001 {
            BASE_RATE
        } else {
            BASE_RATE + (skew_notional / total_notional) * BASE_RATE
        };

        Some((open_interest_usd, funding_rate))
    }

    /// Broadcast price feed to WebSocket subscribers
    pub async fn run_market_data_feed(&self, tx: broadcast::Sender<ServerMessage>) -> Result<()> {
        info!("📈 Perps market data feed starting");
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));

        loop {
            interval.tick().await;

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;

            // Snapshot market list — release lock before async price fetch
            let market_list: Vec<(String, String, f64)> = {
                let markets = self.markets.read().await;
                markets
                    .values()
                    .map(|m| (m.id.clone(), m.base.clone(), m.funding_rate))
                    .collect()
            };

            for (id, symbol, funding_rate) in market_list {
                // Fetch price — no locks held during await
                let last_price = self
                    .tickers
                    .read()
                    .await
                    .get(&id)
                    .map(|t| t.price)
                    .unwrap_or(0.0);
                let price = self.fetch_price(&symbol).await.unwrap_or(last_price);

                if price == 0.0 {
                    continue;
                }

                // ── Push price into ring buffer ───────────────────────────────────
                {
                    let mut history = self.price_history.write().await;
                    let buf = history.entry(id.clone()).or_insert_with(VecDeque::new);
                    buf.push_back((now, price));

                    // Trim entries older than 25h
                    let cutoff_ms = now.saturating_sub(25 * 3600 * 1_000);
                    while buf.front().map(|(t, _)| *t < cutoff_ms).unwrap_or(false) {
                        buf.pop_front();
                    }
                }

                // ── Compute 24h stats ─────────────────────────────────────────────
                let (
                    price_24h_ago,
                    high_24h,
                    low_24h,
                    volume_24h_approx,
                    change_24h,
                    change_pct_24h,
                ) = {
                    let history = self.price_history.read().await;
                    let buf = history.get(&id);
                    let cutoff_24h = now.saturating_sub(24 * 3600 * 1_000);

                    if let Some(buf) = buf {
                        let price_24h_ago = buf
                            .iter()
                            .find(|(t, _)| *t >= cutoff_24h)
                            .map(|(_, p)| *p)
                            .unwrap_or(price);

                        let (high_24h, low_24h, vol_sum) = buf
                            .iter()
                            .filter(|(t, _)| *t >= cutoff_24h)
                            .fold((price, price, 0.0f64), |(h, l, v), (_, p)| {
                                (h.max(*p), l.min(*p), v + p)
                            });

                        let change_24h = price - price_24h_ago;
                        let change_pct_24h = if price_24h_ago > 0.0 {
                            (change_24h / price_24h_ago) * 100.0
                        } else {
                            0.0
                        };

                        // proxy volume until real trade volume is tracked
                        (
                            price_24h_ago,
                            high_24h,
                            low_24h,
                            vol_sum / 1_000.0,
                            change_24h,
                            change_pct_24h,
                        )
                    } else {
                        (price, price, price, 0.0, 0.0, 0.0)
                    }
                };

                let ticker = MarketTicker {
                    market_id: id.clone(),
                    price,
                    price_24h_ago,
                    change_24h,
                    change_pct_24h,
                    volume_24h: volume_24h_approx,
                    high_24h,
                    low_24h,
                    open_interest: {
                        self.market_stats_cache
                            .read()
                            .await
                            .get(&id)
                            .map(|(oi, _)| *oi)
                            .unwrap_or(0.0)
                    },
                    funding_rate: {
                        self.market_stats_cache
                            .read()
                            .await
                            .get(&id)
                            .map(|(_, fr)| *fr)
                            .unwrap_or(funding_rate)
                    },
                    next_funding_ts: now + 28_800_000,
                    timestamp: now,
                };

                self.tickers
                    .write()
                    .await
                    .insert(id.clone(), ticker.clone());
                let _ = tx.send(ServerMessage::TickerUpdate { ticker });
                // Refresh on-chain stats every 30 ticks (~60s)
                {
                    let tick_count = self.tickers.read().await.len();
                    if tick_count % 30 == 0 {
                        if let Some((oi, fr)) = self.fetch_market_stats(&id, price).await {
                            self.market_stats_cache
                                .write()
                                .await
                                .insert(id.clone(), (oi, fr));
                        }
                    }
                }
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
    ) -> Result<(Vec<u8>, u8, String)> {
        use anchor_lang::InstructionData;
        use anchor_lang::ToAccountMetas;
        use solana_sdk::instruction::Instruction;
        use solana_sdk::message::Message;

        let owner_key = Pubkey::from_str(owner)?;
        let program_id = Pubkey::from_str(&self.config.program_id)?;

        // market_id string → padded 16-byte array (same as on-chain)
        let market_id_bytes = market_id.as_bytes();
        let mut market_id_16 = [0u8; 16];
        let len = market_id_bytes.len().min(16);
        market_id_16[..len].copy_from_slice(&market_id_bytes[..len]);

        // Convert feed id (SOL-USDC) → on-chain market id (SOL-PERP) for cache lookup
        let onchain_id = Self::feed_to_onchain(market_id).unwrap_or(market_id);
        let mut cache_key = [0u8; 16];
        let b = onchain_id.as_bytes();
        cache_key[..b.len().min(16)].copy_from_slice(&b[..b.len().min(16)]);

       let (market_pda, _) =
            Pubkey::find_program_address(&[b"market", &cache_key], &program_id);
        let (margin_pda, _) = Pubkey::find_program_address(
            &[b"margin", &cache_key, owner_key.as_ref()],
            &program_id,
        );

        // Find first unused nonce — scan PDAs until one has no on-chain account
        let (nonce, position_pda) = {
            let mut found = None;
         for n in 0u8..=255 {
                let (pda, _) = Pubkey::find_program_address(
                    &[b"position", &cache_key, owner_key.as_ref(), &[n]],
                    &program_id,
                );
                match self.rpc.get_account(&pda).await {
                    Err(_) => {
                        // Account doesn't exist — this nonce is free
                        found = Some((n, pda));
                        break;
                    }
                    Ok(acc) if acc.data.is_empty() => {
                        found = Some((n, pda));
                        break;
                    }
                    Ok(_) => continue, // account exists — try next nonce
                }
            }
            found.context("no free position nonce found (all 256 used?)")?
        };

        let cached = self
            .market_cache
            .get(&cache_key)
            .context("market not in cache — ensure register_markets ran at startup")?;

        let quote_mint = cached.quote_mint;
        let user_token_account =
            anchor_spl::associated_token::get_associated_token_address(&owner_key, &quote_mint);
        let create_ata_ix =
anchor_spl::associated_token::spl_associated_token_account::instruction::create_associated_token_account_idempotent(                &owner_key,
                &owner_key,
                &quote_mint,
                &anchor_spl::token::spl_token::ID,
            );

let size_u64 = (size * 1_000_000.0) as u64;
        let leverage_bps = (leverage * 100.0) as u64; // e.g. 10x → 1000 bps

        let accounts = soldex_perps::accounts::InitPosition {
            user: owner_key,
            market: market_pda,
            margin: margin_pda,
            position: position_pda,
            system_program: solana_sdk::system_program::id(),
            rent: solana_sdk::sysvar::rent::id(),
        };

        let params = soldex_perps::InitPositionParams {
            market_id: cache_key,
            side: match side {
                PositionSide::Long => soldex_perps::state::PositionSide::Long,
                PositionSide::Short => soldex_perps::state::PositionSide::Short,
            },
            size: size_u64,
            leverage_bps,
            collateral: collateral_lamports,
            nonce,
        };
        let data = soldex_perps::instruction::InitPosition { params };

        let ix = Instruction {
            program_id,
            accounts: accounts.to_account_metas(None),
            data: data.data(),
        };

        let recent_blockhash = self.rpc.get_latest_blockhash().await?;

        let msg = Message::new(&[create_ata_ix, ix], Some(&owner_key));
        let mut tx = Transaction::new_unsigned(msg);
        tx.message.recent_blockhash = recent_blockhash;

        info!(
            "Built open_position tx: {} {size} {market_id} x{leverage} pos={position_pda}",
            if matches!(side, PositionSide::Long) {
                "LONG"
            } else {
                "SHORT"
            },
        );

        // Serialize to base64-ready bytes — client deserializes, signs, submits
        Ok((
            bincode::serialize(&tx).context("serialize tx")?,
            nonce,
            position_pda.to_string(),
        ))
    }

    pub async fn build_deposit_collateral_tx(
        &self,
        owner: &str,
        market_id: &str,
        amount: u64,
    ) -> Result<Vec<u8>> {
        use anchor_lang::InstructionData;
        use anchor_lang::ToAccountMetas;
        use solana_sdk::instruction::Instruction;
        use solana_sdk::message::Message;

        let owner_key = Pubkey::from_str(owner)?;
        let program_id = Pubkey::from_str(&self.config.program_id)?;

        let mut market_id_16 = [0u8; 16];
        let bytes = market_id.as_bytes();
        market_id_16[..bytes.len().min(16)].copy_from_slice(&bytes[..bytes.len().min(16)]);

        let onchain_id = Self::feed_to_onchain(market_id).unwrap_or(market_id);
        let mut cache_key = [0u8; 16];
        let b = onchain_id.as_bytes();
        cache_key[..b.len().min(16)].copy_from_slice(&b[..b.len().min(16)]);

       let (market_pda, _) =
            Pubkey::find_program_address(&[b"market", &cache_key], &program_id);
        let (margin_pda, _) = Pubkey::find_program_address(
            &[b"margin", &cache_key, owner_key.as_ref()],
            &program_id,
        );

        // Look up market from startup cache — avoids RPC fetch on delegated account
        let cached = self
            .market_cache
            .get(&cache_key)
            .context("market not in cache — ensure register_markets ran at startup")?;
        let quote_mint = cached.quote_mint;
        let (vault, _) = Pubkey::find_program_address(&[b"vault", &cache_key], &program_id);
        // user's ATA for quote_mint
        let user_token_account =
            anchor_spl::associated_token::get_associated_token_address(&owner_key, &quote_mint);

        // Create the ATA if it doesn't exist yet — idempotent, safe to include always
        let create_ata_ix =
anchor_spl::associated_token::spl_associated_token_account::instruction::create_associated_token_account_idempotent(                &owner_key,
                &owner_key,
                &quote_mint,
                &anchor_spl::token::spl_token::ID,
            );

        let accounts = soldex_perps::accounts::DepositCollateral {
            owner: owner_key,
            margin: margin_pda,
            user_token_account,
            vault,
            token_program: anchor_spl::token::spl_token::ID,
            system_program: solana_sdk::system_program::id(),
        };

      let data = soldex_perps::instruction::DepositCollateral {
            market_id: cache_key,
            amount,
        };

        let ix = Instruction {
            program_id,
            accounts: accounts.to_account_metas(None),
            data: data.data(),
        };

        let recent_blockhash = self.rpc.get_latest_blockhash().await?;
        let msg = Message::new(&[create_ata_ix, ix], Some(&owner_key));
        let mut tx = Transaction::new_unsigned(msg);
        tx.message.recent_blockhash = recent_blockhash;

        info!(
            "Built deposit_collateral tx: {} lamports for {}",
            amount, owner
        );
        Ok(bincode::serialize(&tx).context("serialize tx")?)
    }

    pub async fn get_latest_blockhash(&self) -> Result<Hash> {
        let hash = self.rpc.get_latest_blockhash().await?;
        Ok(hash)
    }

    pub fn get_rpc_client(&self) -> Arc<RpcClient> {
        self.rpc.clone()
    }
}
