use crate::ctx_accounts::{FundingTickEr, FundingSettled};
use crate::errors::SoldexError;
use crate::state::{MarketState, FUNDING_INTERVAL_SECS};
use anchor_lang::prelude::*;

const FUNDING_SCALE: i128     = 1_000_000_000;
const BASE_FUNDING_RATE: i128 = 100_000_000;
const MIN_OI_THRESHOLD: i128  = 1_000;

pub fn handler(ctx: Context<FundingTickEr>, mark_price: u64) -> Result<()> {
    // Manually deserialize — market is AccountInfo (delegated, owned by delegation program on ER)
    let mut data = ctx.accounts.market.try_borrow_mut_data()?;
    let mut market: MarketState = AnchorDeserialize::deserialize(&mut &data[8..])?;

    let clock = Clock::get()?;
    require!(
        clock.unix_timestamp >= market.last_funding_ts
            .checked_add(FUNDING_INTERVAL_SECS)
            .ok_or(SoldexError::Overflow)?,
        SoldexError::FundingAlreadySettled
    );

    let long_oi  = market.long_open_interest  as i128;
    let short_oi = market.short_open_interest as i128;
    let total_oi = long_oi.checked_add(short_oi).ok_or(SoldexError::Overflow)?;
    let total_notional = total_oi
        .checked_mul(mark_price as i128).ok_or(SoldexError::Overflow)?
        .checked_div(1_000_000).ok_or(SoldexError::Overflow)?;
    let oi_skew = long_oi.checked_sub(short_oi).ok_or(SoldexError::Overflow)?;
    let skew_notional = oi_skew
        .checked_mul(mark_price as i128).ok_or(SoldexError::Overflow)?
        .checked_div(1_000_000).ok_or(SoldexError::Overflow)?;
    let skew_rate = if total_notional.abs() < MIN_OI_THRESHOLD {
        0
    } else {
        skew_notional
            .checked_mul(FUNDING_SCALE).ok_or(SoldexError::Overflow)?
            .checked_div(total_notional).ok_or(SoldexError::Overflow)?
    };
    let funding_rate = BASE_FUNDING_RATE
        .checked_add(skew_rate).ok_or(SoldexError::Overflow)?;

    market.cumulative_funding_long = market.cumulative_funding_long
        .checked_add(funding_rate).ok_or(SoldexError::Overflow)?;
    market.cumulative_funding_short = market.cumulative_funding_short
        .checked_sub(funding_rate).ok_or(SoldexError::Overflow)?;
    market.last_funding_ts = clock.unix_timestamp;

    // Serialize back into the account data
    let dst = &mut data[8..];
    AnchorSerialize::serialize(&market, &mut &mut dst[..])?;

    emit!(FundingSettled {
        market_id: market.market_id,
        funding_rate,
        timestamp: clock.unix_timestamp,
    });
    Ok(())
}