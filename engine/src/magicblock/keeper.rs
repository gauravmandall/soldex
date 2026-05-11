//! Keeper bot — funding tick and liquidation loops over all delegated positions.
//! Keeper can only sign: liquidate_er, close_position_er. Never open/withdraw.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use dashmap::DashMap;
use solana_sdk::pubkey::Pubkey;
use tokio::time::interval;
use tracing::{error, info, warn};

use super::session::ErSession;
use super::tx_builder::ErTxBuilder;
use crate::perps::PerpsEngine;
use crate::utils::BlockhashCache;

pub struct KeeperConfig {
    pub funding_interval_secs: u64,
    pub liquidation_scan_interval_secs: u64,
    /// Minimum margin ratio in bps before liquidation (500 = 5%)
    pub min_margin_ratio_bps: u64,
}

impl Default for KeeperConfig {
    fn default() -> Self {
        Self {
            funding_interval_secs: 60,
            liquidation_scan_interval_secs: 5,
            min_margin_ratio_bps: 600,
        }
    }
}

/// Cached per-market data needed to build liquidation txs
struct MarketMeta {
    pub market_pda: Pubkey, // seeds: [b"market", market_id]
    pub price_feed: Pubkey, // Pyth feed — passed as account to liquidate ix
}

pub struct KeeperBot {
    registry: Arc<crate::magicblock::SessionRegistry>,
    tx_builder: Arc<ErTxBuilder>,
    markets: Arc<DashMap<[u8; 16], MarketMeta>>,
    config: KeeperConfig,
    blockhash_cache: Arc<BlockhashCache>,
    perps: Arc<PerpsEngine>,
}

impl KeeperBot {
    pub fn new(
        registry: Arc<crate::magicblock::SessionRegistry>,
        tx_builder: Arc<ErTxBuilder>,
        blockhash_cache: Arc<BlockhashCache>,
        config: KeeperConfig,
        perps: Arc<PerpsEngine>,
    ) -> Self {
        Self {
            registry,
            tx_builder,
            markets: Arc::new(DashMap::new()),
            blockhash_cache,
            config,
            perps,
        }
    }

    /// Register a market at startup. price_feed must match MarketState.price_feed on-chain.
    pub fn register_market(&self, market_id: [u8; 16], market_pda: Pubkey, price_feed: Pubkey) {
        info!(
            market = ?std::str::from_utf8(&market_id).unwrap_or("?"),
            %market_pda, %price_feed, "Market registered"
        );
        self.markets.insert(
            market_id,
            MarketMeta {
                market_pda,
                price_feed,
            },
        );
    }

    pub fn list_markets(&self) -> Vec<([u8; 16], Pubkey)> {
        self.markets
            .iter()
            .map(|entry| (*entry.key(), entry.value().market_pda))
            .collect()
    }

    /// Spawn funding + liquidation loops. Call once at engine startup.
    pub async fn run(self: Arc<Self>) {
        let funding_task = tokio::spawn({
            let bot = self.clone();
            async move { bot.funding_loop().await }
        });
        let liq_task = tokio::spawn({
            let bot = self.clone();
            async move { bot.liquidation_loop().await }
        });
        let price_task = tokio::spawn({
            let bot = self.clone();
            async move { bot.price_pump_loop().await }
        });
        tokio::join!(funding_task, liq_task, price_task);
    }

    // ── Funding loop (stubbed) ────────────────────────────────────────────────

    async fn funding_loop(self: Arc<Self>) {
        let mut ticker = interval(Duration::from_secs(self.config.funding_interval_secs));
        ticker.tick().await; // skip first immediate tick
        loop {
            ticker.tick().await;
            for entry in self.markets.iter() {
                let market_id = *entry.key();
                let market_pda = entry.value().market_pda;
                if let Err(e) = self.tick_funding(market_id, market_pda).await {
                    error!(market = ?std::str::from_utf8(&market_id).unwrap_or("?"), "funding tick failed: {e}");
                }
            }
        }
    }
    async fn tick_funding(&self, market_id: [u8; 16], market_pda: Pubkey) -> Result<()> {
        let mark_price_u64 = {
            let tickers = self.perps.tickers.read().await;
            let market_str = std::str::from_utf8(&market_id)
                .unwrap_or("")
                .trim_matches('\0');
            let feed_key = match market_str {
                "SOL-PERP" => "SOL-USDC",
                "BTC-PERP" => "BTC-USDC",
                "ETH-PERP" => "ETH-USDC",
                "JUP-PERP" => "JUP-USDC",
                other => other,
            };
            match tickers.get(feed_key) {
                Some(t) if t.price > 0.0 => (t.price * 1_000_000.0) as u64,
                _ => {
                    info!(market = ?market_str, "funding tick skipped — no price cached yet");
                    return Ok(());
                }
            }
        };

        let session = match self.registry.iter_sessions().into_iter().next() {
            Some((_owner, s)) => s,
            None => {
                info!(market = ?std::str::from_utf8(&market_id).unwrap_or("?"), "funding tick skipped — no active sessions");
                return Ok(());
            }
        };

        let blockhash = session.recent_blockhash().await?;
        let tx = self
            .tx_builder
            .funding_tick_er_tx(&market_pda, mark_price_u64, blockhash)?;

        let start = std::time::Instant::now();

        match session.submit(tx).await {
            Ok(sig) => {
                let latency = start.elapsed().as_millis();
                info!(
                    market = ?std::str::from_utf8(&market_id).unwrap_or("?"),
                    %sig,
                    latency_ms = latency,
                    "funding tick confirmed"
                );
            }
            Err(e)
                if format!("{e:#}").contains("FundingAlreadySettled")
                    || format!("{e:#}").contains("0x1778") =>
            {
                info!(market = ?std::str::from_utf8(&market_id).unwrap_or("?"), "funding already settled for this interval");
            }
            Err(e) => error!(
                market = ?std::str::from_utf8(&market_id).unwrap_or("?"),
                "funding tick tx failed: {e}"
            ),
        }
        Ok(())
    }

    // liquidation loop — checks margin on cached price every 5s, program re-validates on-chain
    async fn liquidation_loop(self: Arc<Self>) {
        let mut ticker = interval(Duration::from_secs(
            self.config.liquidation_scan_interval_secs,
        ));
        loop {
            ticker.tick().await;
            // Iterate all active Private ER sessions
            for (_owner, session) in self.registry.iter_sessions() {
                let positions: Vec<_> = session
                    .delegated
                    .iter()
                    .map(|e| e.value().clone())
                    .collect();
                for pos in positions {
                    if pos.last_mark_price == 0 {
                        continue;
                    }
                    if let Err(e) = self.check_and_liquidate(&pos, pos.last_mark_price).await {
                        error!(position = %pos.position_pda, "liquidation check failed: {e}");
                    }
                }
            }
        }
    }

    /// Local margin check — no RPC. Fires liquidate tx if ratio < threshold.
    async fn check_and_liquidate(
        &self,
        pos: &super::session::DelegatedPosition,
        mark_price: u64,
    ) -> Result<()> {
        if pos.size == 0 {
            return Ok(());
        }

        // notional = size × price ÷ 1e6
        let notional = (pos.size as u128).saturating_mul(mark_price as u128) / 1_000_000;
        if notional == 0 {
            return Ok(());
        }

        let price_delta: i128 = if pos.is_long {
            mark_price as i128 - pos.entry_price as i128
        } else {
            pos.entry_price as i128 - mark_price as i128
        };
        let pnl = (pos.size as i128) * price_delta / 1_000_000;
        let equity = (pos.collateral as i128).saturating_add(pnl).max(0) as u128;
        let margin_ratio_bps = (equity.saturating_mul(10_000) / notional) as u64;

        if margin_ratio_bps < self.config.min_margin_ratio_bps {
            warn!(position = %pos.position_pda, margin_ratio_bps, mark_price, "undercollateralised — liquidating");
            self.execute_liquidation(pos, mark_price).await?;
        }
        Ok(())
    }

    /// Build + submit liquidate tx via TEE RPC. Program re-checks margin on-chain.
    async fn execute_liquidation(
        &self,
        pos: &super::session::DelegatedPosition,
        mark_price: u64,
    ) -> Result<()> {
        let meta = match self.markets.get(&pos.market_id) {
            Some(m) => m,
            None => {
                warn!(position = %pos.position_pda, "market not registered — skipping");
                return Ok(());
            }
        };

        let margin_pda = self
            .tx_builder
            .derive_margin_pda(&pos.market_id, &pos.owner);
        let session = match self.registry.get(&pos.owner) {
            Some(s) => s,
            None => {
                warn!(position = %pos.position_pda, "no session for owner — skipping liquidation");
                return Ok(());
            }
        };
        let blockhash = session.recent_blockhash().await?;
        let tx = self.tx_builder.liquidate_er_tx(
            &pos.position_pda,
            &meta.market_pda,
            &pos.owner,
            pos.market_id,
            pos.nonce,
            mark_price,
            blockhash,
        )?;
        match session.submit(tx).await {
            Ok(sig) => {
                info!(position = %pos.position_pda, %sig, "liquidation confirmed");
                session.remove_delegation(&pos.position_pda);
                if session.delegated.is_empty() {
                    self.registry.remove(&pos.owner);
                }
            }
            Err(e)
                if e.to_string().contains("NotLiquidatable")
                    || e.to_string().contains("0x1776")
                    || e.to_string().contains("0x1776") =>
            {
                info!(position = %pos.position_pda, "position not liquidatable on-chain (equity above maintenance)");
            }
            Err(e) => error!(position = %pos.position_pda, "liquidation tx failed: {e:#}"),
        }
        Ok(())
    }

    // price pump — pushes fresh mark prices into delegated position cache every 2s
    async fn price_pump_loop(self: Arc<Self>) {
        let mut ticker = interval(Duration::from_secs(2));
        loop {
            ticker.tick().await;

            let tickers = self.perps.tickers.read().await;

            // Nothing to do if price feed hasn't populated yet
            if tickers.is_empty() {
                continue;
            }

            for (_owner, session) in self.registry.iter_sessions() {
                // Snapshot PDAs so we don't hold DashMap ref across await
                let pdas: Vec<_> = session
                    .delegated
                    .iter()
                    .map(|e| (e.value().position_pda, e.value().market_id))
                    .collect();

                for (pda, market_id) in pdas {
                    let onchain_str = std::str::from_utf8(&market_id)
                        .unwrap_or("")
                        .trim_matches('\0');
                    // Map on-chain market_id → feed name used in tickers cache
                    let feed_str = match onchain_str {
                        "SOL-PERP" => "SOL-USDC",
                        "BTC-PERP" => "BTC-USDC",
                        "ETH-PERP" => "ETH-USDC",
                        "JUP-PERP" => "JUP-USDC",
                        other => other, // pass through if already feed format
                    };
                    let price_f64 = match tickers.get(feed_str) {
                        Some(t) if t.price > 0.0 => t.price,
                        _ => continue,
                    };

                    // Convert to u64 micro-units (6 decimals) matching
                    // check_and_liquidate's  size * mark_price / 1_000_000
                    let mark_price_u64 = (price_f64 * 1_000_000.0) as u64;

                    // Read current collateral/size without holding mut ref
                    let (cur_size, cur_collateral) = session
                        .delegated
                        .get(&pda)
                        .map(|p| (p.size, p.collateral))
                        .unwrap_or((0, 0));

                    if cur_size == 0 {
                        continue; // position already closed, skip
                    }

                    session.update_position_cache(&pda, cur_size, mark_price_u64, cur_collateral);
                }
            }
        }
    }
}
