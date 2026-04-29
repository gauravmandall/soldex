//! magicblock/keeper.rs
//! Two background loops:
//!   1. Funding tick — every N seconds (disabled until funding_tick_er is added to program)
//!   2. Liquidation scanner — every 5s checks margin health (disabled until liquidate_er is added)

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use dashmap::DashMap;
use solana_sdk::pubkey::Pubkey;
use tokio::time::interval;
use tracing::{error, info, warn};

use super::session::ErSession;
use super::tx_builder::ErTxBuilder;
use crate::utils::BlockhashCache;

pub struct KeeperConfig {
    pub funding_interval_secs: u64,
    pub liquidation_scan_interval_secs: u64,
    pub min_margin_ratio_bps: u64,
}

impl Default for KeeperConfig {
    fn default() -> Self {
        Self {
            funding_interval_secs: 60,
            liquidation_scan_interval_secs: 5,
            min_margin_ratio_bps: 500,
        }
    }
}

pub struct KeeperBot {
    session: Arc<ErSession>,
    tx_builder: Arc<ErTxBuilder>,
    markets: Arc<DashMap<[u8; 16], Pubkey>>, // market_id → market_state_pda
    config: KeeperConfig,
    blockhash_cache: Arc<BlockhashCache>,
}

impl KeeperBot {
    pub fn new(
        session: Arc<ErSession>,
        tx_builder: Arc<ErTxBuilder>,
        blockhash_cache: Arc<BlockhashCache>,
        config: KeeperConfig,
    ) -> Self {
        Self {
            session,
            tx_builder,
            markets: Arc::new(DashMap::new()),
            blockhash_cache,
            config,
        }
    }

    pub fn register_market(&self, market_id: [u8; 16], market_state_pda: Pubkey) {
        self.markets.insert(market_id, market_state_pda);
    }

    /// Start both keeper loops. Call once at engine startup.
    pub async fn run(self: Arc<Self>) {
        let funding_task = tokio::spawn({
            let bot = self.clone();
            async move { bot.funding_loop().await }
        });

        let liq_task = tokio::spawn({
            let bot = self.clone();
            async move { bot.liquidation_loop().await }
        });

        tokio::join!(funding_task, liq_task);
    }

    // ── Funding loop ──────────────────────────────────────────────────────────

    async fn funding_loop(self: Arc<Self>) {
        let mut ticker = interval(Duration::from_secs(self.config.funding_interval_secs));
        ticker.tick().await; // skip immediate first tick

        loop {
            ticker.tick().await;

            for entry in self.markets.iter() {
                let market_id = *entry.key();
                let market_pda = *entry.value();

                if let Err(e) = self.tick_funding(market_id, market_pda).await {
                    error!(
                        "funding tick failed for {:?}: {}",
                        std::str::from_utf8(&market_id).unwrap_or("?"),
                        e
                    );
                }
            }
        }
    }

    async fn tick_funding(&self, market_id: [u8; 16], _market_pda: Pubkey) -> Result<()> {
        // TODO: uncomment when funding_tick_er instruction is added to program
        // let blockhash = self.blockhash_cache.get().await;
        // let tx = self.tx_builder.funding_tick_er_tx(&_market_pda, market_id, blockhash)?;
        // let sig = self.session.submit_fire_and_forget(tx).await?;
        info!(
            "funding tick skipped for {:?} — funding_tick_er not implemented yet",
            std::str::from_utf8(&market_id).unwrap_or("?")
        );
        Ok(())
    }

    // ── Liquidation scanner ───────────────────────────────────────────────────

    async fn liquidation_loop(self: Arc<Self>) {
        let mut ticker = interval(Duration::from_secs(self.config.liquidation_scan_interval_secs));

        loop {
            ticker.tick().await;

            let positions: Vec<_> = self
                .session
                .delegated
                .iter()
                .map(|e| e.value().clone())
                .collect();

            for pos in positions {
                // TODO: replace with real price feed when price_feed module is added
                // For now skip liquidation check — no price source available
                info!(
                    "liquidation check skipped for {} — price_feed not implemented yet",
                    pos.position_pda
                );
            }
        }
    }

    async fn check_and_liquidate(
        &self,
        pos: &super::session::DelegatedPosition,
        mark_price: u64,
    ) -> Result<()> {
        if pos.size == 0 {
            return Ok(());
        }

        let notional = (pos.size as u128)
            .saturating_mul(mark_price as u128)
            / 1_000_000;

        if notional == 0 {
            return Ok(());
        }

        let margin_ratio_bps =
            ((pos.collateral as u128).saturating_mul(10_000) / notional) as u64;

        if margin_ratio_bps < self.config.min_margin_ratio_bps {
            warn!(
                "LIQUIDATING {} margin_ratio={}bps mark_price={}",
                pos.position_pda, margin_ratio_bps, mark_price
            );
            self.execute_liquidation(pos, mark_price).await?;
        }

        Ok(())
    }

    async fn execute_liquidation(
        &self,
        pos: &super::session::DelegatedPosition,
        _mark_price: u64,
    ) -> Result<()> {
        let _market_pda = match self.markets.get(&pos.market_id) {
            Some(p) => *p,
            None => anyhow::bail!("market PDA not found for {:?}", pos.market_id),
        };

        // TODO: uncomment when liquidate_er instruction is added to program
        // let blockhash = self.blockhash_cache.get().await;
        // let tx = self.tx_builder.liquidate_er_tx(
        //     &pos.position_pda,
        //     &_market_pda,
        //     pos.market_id,
        //     _mark_price,
        //     0i128,
        //     blockhash,
        // )?;
        // let sig = self.session.submit(tx).await?;
        // self.session.remove_delegation(&pos.position_pda);

        info!("liquidate_er not implemented yet — skipping liquidation for {}", pos.position_pda);
        Ok(())
    }
}