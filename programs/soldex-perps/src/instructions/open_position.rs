use anchor_lang::prelude::*;
use crate::errors::SoldexError;
use crate::state::{Position, PositionSide};
use crate::{OpenPosition, OpenPositionParams};

pub fn handler(ctx: Context<OpenPosition>, params: OpenPositionParams) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let margin = &mut ctx.accounts.margin;
    let position = &mut ctx.accounts.position;

    require!(
        params.leverage_bps > 0 && params.leverage_bps <= market.max_leverage_bps,
        SoldexError::LeverageExceeded
    );
    require!(params.size >= market.lot_size, SoldexError::BelowMinLotSize);
    require!(margin.collateral >= params.collateral, SoldexError::InsufficientCollateral);

    let mark_price: u64 = 150_000_000; // stub — replace with Pyth

    let notional = params.size
        .checked_mul(mark_price)
        .ok_or(SoldexError::Overflow)? / 1_000_000;
    let required_margin = notional
        .checked_mul(market.initial_margin_bps as u64)
        .ok_or(SoldexError::Overflow)? / 10_000;
    require!(params.collateral >= required_margin, SoldexError::InsufficientCollateral);

    match params.side {
        PositionSide::Long  => market.long_open_interest  = market.long_open_interest.checked_add(params.size).ok_or(SoldexError::Overflow)?,
        PositionSide::Short => market.short_open_interest = market.short_open_interest.checked_add(params.size).ok_or(SoldexError::Overflow)?,
    }

    margin.collateral = margin.collateral
        .checked_sub(params.collateral)
        .ok_or(SoldexError::InsufficientCollateral)?;

    let clock = Clock::get()?;
    position.set_inner(Position {
        market_id: market.market_id,
        owner: ctx.accounts.user.key(),
        side: params.side,
        size: params.size,
        entry_price: mark_price,
        collateral: params.collateral,
        leverage_bps: params.leverage_bps,
        entry_funding_index: match params.side {
            PositionSide::Long  => market.cumulative_funding_long,
            PositionSide::Short => market.cumulative_funding_short,
        },
        opened_at: clock.unix_timestamp,
        is_open: true,
        is_delegated: false,
        delegated_at: 0,
        last_update_ts: 0,
        bump: ctx.bumps.position,
        _reserved: [0u8; 32],
    });

    emit!(PositionOpened {
        user: ctx.accounts.user.key(),
        market_id: market.market_id,
        side: params.side,
        size: params.size,
        entry_price: mark_price,
        collateral: params.collateral,
        leverage_bps: params.leverage_bps,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct PositionOpened {
    pub user: Pubkey,
    pub market_id: [u8; 16],
    pub side: PositionSide,
    pub size: u64,
    pub entry_price: u64,
    pub collateral: u64,
    pub leverage_bps: u64,
    pub timestamp: i64,
}
