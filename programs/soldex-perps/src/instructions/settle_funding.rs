use anchor_lang::prelude::*;
use crate::ctx_accounts::{SettleFunding, FundingSettled};
use crate::errors::SoldexError;
use crate::state::FUNDING_INTERVAL_SECS;
use crate::price::get_mark_price;


// Funding index scaled by 1e9 for precision
const FUNDING_SCALE: i128       = 1_000_000_000;
// Base funding rate per interval: 0.01% = 100_000_000 in 1e9 scale
const BASE_FUNDING_RATE: i128   = 100_000_000;
// Min total OI to avoid division by zero in skew calc
const MIN_OI_THRESHOLD: i128    = 1_000;

pub fn handler(ctx: Context<SettleFunding>) -> Result<()> {
    let mark_price = get_mark_price(&ctx.accounts.price_feed)?;
    let market = &mut ctx.accounts.market;
    let clock  = Clock::get()?;

    // ── Guard: enforce funding interval ───────────────────────────────────────
    require!(
        clock.unix_timestamp >= market.last_funding_ts
            .checked_add(FUNDING_INTERVAL_SECS)
            .ok_or(SoldexError::Overflow)?,
        SoldexError::FundingAlreadySettled
    );

    let long_oi  = market.long_open_interest  as i128;
    let short_oi = market.short_open_interest as i128;

    // ── Total OI notional (USD, scaled 1e6) ───────────────────────────────────
    // total_notional = (long_oi + short_oi) × mark_price ÷ 1e6
    let total_oi: i128 = long_oi
        .checked_add(short_oi)
        .ok_or(SoldexError::Overflow)?;

    let total_notional: i128 = total_oi
        .checked_mul(mark_price as i128)
        .ok_or(SoldexError::Overflow)?
        .checked_div(1_000_000)
        .ok_or(SoldexError::Overflow)?;

    // ── OI skew notional (signed USD, scaled 1e6) ─────────────────────────────
    // Positive = more longs than shorts → longs pay funding to shorts
    let oi_skew: i128 = long_oi
        .checked_sub(short_oi)
        .ok_or(SoldexError::Overflow)?;

    let skew_notional: i128 = oi_skew
        .checked_mul(mark_price as i128)
        .ok_or(SoldexError::Overflow)?
        .checked_div(1_000_000)
        .ok_or(SoldexError::Overflow)?;

    // ── Skew rate (scaled 1e9) ────────────────────────────────────────────────
    // skew_rate = skew_notional ÷ total_notional × FUNDING_SCALE
    // If total OI too small, skew_rate = 0 to avoid division by zero
    let skew_rate: i128 = if total_notional.abs() < MIN_OI_THRESHOLD {
        0
    } else {
        skew_notional
            .checked_mul(FUNDING_SCALE)
            .ok_or(SoldexError::Overflow)?
            .checked_div(total_notional)
            .ok_or(SoldexError::Overflow)?
    };

    // ── Final funding rate (scaled 1e9) ───────────────────────────────────────
    // base_rate always positive — prevents zero funding in balanced markets
    let funding_rate: i128 = BASE_FUNDING_RATE
        .checked_add(skew_rate)
        .ok_or(SoldexError::Overflow)?;

    // ── Update cumulative indexes ─────────────────────────────────────────────
    // Longs pay when funding_rate > 0, receive when < 0
    // Shorts receive when funding_rate > 0, pay when < 0
    market.cumulative_funding_long = market.cumulative_funding_long
        .checked_add(funding_rate)
        .ok_or(SoldexError::Overflow)?;

    market.cumulative_funding_short = market.cumulative_funding_short
        .checked_sub(funding_rate)
        .ok_or(SoldexError::Overflow)?;

    // ── Update timestamp ──────────────────────────────────────────────────────
    market.last_funding_ts = clock.unix_timestamp;

    // Use FundingSettled event shape defined in ctx_accounts.rs
    emit!(FundingSettled {
        market_id:    market.market_id,
        funding_rate,
        timestamp:    clock.unix_timestamp,
    });

    Ok(())
}