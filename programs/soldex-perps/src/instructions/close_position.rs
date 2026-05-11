use crate::ctx_accounts::{ClosePosition, ClosePositionParams, PositionClosed};
use crate::errors::SoldexError;
use crate::state::PositionSide;
use crate::price::get_mark_price;
use anchor_lang::prelude::*;

pub fn handler(ctx: Context<ClosePosition>, _params: ClosePositionParams) -> Result<()> {
    let mark_price = get_mark_price(&ctx.accounts.price_feed)?;
    let position = &mut ctx.accounts.position;
    let market = &mut ctx.accounts.market;
    let margin = &mut ctx.accounts.margin;

    // ── Guards ────────────────────────────────────────────────────────────────
    require!(position.is_open, SoldexError::PositionNotOpen);
    require!(!position.is_delegated, SoldexError::AlreadyDelegated);
    require!(
        position.owner == ctx.accounts.user.key(),
        SoldexError::NotPositionOwner
    );
    require!(
        position.market_id == market.market_id,
        SoldexError::InvalidSeeds
    );

    let size = position.size;
    let entry_price = position.entry_price;
    let collateral = position.collateral;

    let price_delta: i128 = match position.side {
        PositionSide::Long => mark_price as i128 - entry_price as i128,
        PositionSide::Short => entry_price as i128 - mark_price as i128,
    };
    let raw_pnl: i128 = (size as i128)
        .checked_mul(price_delta)
        .ok_or(SoldexError::Overflow)?
        .checked_div(1_000_000)
        .ok_or(SoldexError::Overflow)?;

    
    let current_index: i128 = match position.side {
        PositionSide::Long => market.cumulative_funding_long,
        PositionSide::Short => market.cumulative_funding_short,
    };
    let funding_delta: i128 = current_index
        .checked_sub(position.entry_funding_index)
        .ok_or(SoldexError::Overflow)?;

    let funding_pnl: i128 = (size as i128)
        .checked_mul(funding_delta)
        .ok_or(SoldexError::Overflow)?
        .checked_div(1_000_000)
        .ok_or(SoldexError::Overflow)?;

    let total_pnl_i128: i128 = raw_pnl
        .checked_add(funding_pnl)
        .ok_or(SoldexError::Overflow)?;
    
    let total_pnl: i64 = total_pnl_i128
        .try_into()
        .map_err(|_| SoldexError::Overflow)?;

    let notional: u64 = (size as u128)
        .checked_mul(mark_price as u128)
        .ok_or(SoldexError::Overflow)?
        .checked_div(1_000_000)
        .ok_or(SoldexError::Overflow)? as u64;

    let fee: u64 = (notional as u128)
        .checked_mul(market.taker_fee_bps as u128)
        .ok_or(SoldexError::Overflow)?
        .checked_div(10_000)
        .ok_or(SoldexError::Overflow)? as u64;

    let new_collateral: i64 = (margin.collateral as i64)
        .checked_add(total_pnl)
        .ok_or(SoldexError::Overflow)?;

    let after_pnl: u64 = if new_collateral < 0 {
        0
    } else {
        new_collateral as u64
    };

    margin.collateral = after_pnl.saturating_sub(fee);

    market.fees_collected = market
        .fees_collected
        .checked_add(fee)
        .ok_or(SoldexError::Overflow)?;

    match position.side {
        PositionSide::Long => {
            market.long_open_interest = market
                .long_open_interest
                .checked_sub(size)
                .ok_or(SoldexError::Overflow)?;
        }
        PositionSide::Short => {
            market.short_open_interest = market
                .short_open_interest
                .checked_sub(size)
                .ok_or(SoldexError::Overflow)?;
        }
    }
    position.size = 0;
    position.collateral = 0;
    position.is_open = false;

   emit!(PositionClosed {
        owner: ctx.accounts.user.key(),
        market_id: market.market_id,
        pnl: total_pnl,
        mark_price: mark_price,
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_long_pnl_profit() {
        let size: i128 = 1_000_000;
        let entry_price: i128 = 100_000_000;
        let mark_price: i128 = 150_000_000;
        let price_delta = mark_price - entry_price;
        let raw_pnl = size * price_delta / 1_000_000;
        assert_eq!(raw_pnl, 50_000_000);
    }

    #[test]
    fn test_short_pnl_loss() {
        let size: i128 = 1_000_000;
        let entry_price: i128 = 100_000_000;
        let mark_price: i128 = 150_000_000;
        let price_delta = entry_price - mark_price;
        let raw_pnl = size * price_delta / 1_000_000;
        assert_eq!(raw_pnl, -50_000_000);
    }

    #[test]
    fn test_liquidation_condition() {
        let collateral: i128 = 10_000_000;
        let size: i128 = 1_000_000;
        let mark: i128 = 150_000_000;
        let entry: i128 = 100_000_000;
        let raw_pnl = size * (entry - mark) / 1_000_000;
        let equity = collateral + raw_pnl;
        let notional = size * mark / 1_000_000;
        let maintenance = notional * 250 / 10_000;
        assert!(equity < maintenance);
    }
}

