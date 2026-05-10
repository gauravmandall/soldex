// Runs on ER. Liquidates undercollateralised position and subtracts OI from delegated market.

use anchor_lang::prelude::*;
use crate::errors::SoldexError;
use crate::state::{Position, PositionSide, MarketState};
use crate::ctx_accounts::{LiquidateEr, PositionLiquidated};

pub fn liquidate_er_handler(
    ctx: Context<LiquidateEr>,
    mark_price: u64,
    _market_id: [u8; 16],
    _nonce: u8,
) -> Result<()> {
    require!(mark_price > 0, SoldexError::InvalidOraclePrice);

    let mut position = {
        let data = ctx.accounts.position.try_borrow_data()?;
        require!(data.len() > 8, SoldexError::PositionNotFound);
        Position::try_deserialize(&mut &data[..])?
    };

    require!(position.is_open,      SoldexError::PositionNotOpen);
    require!(position.is_delegated, SoldexError::NotDelegated);
    require!(
        position.owner == ctx.accounts.owner.key(),
        SoldexError::InvalidSeeds
    );

    let mut market = {
        let data = ctx.accounts.market.try_borrow_data()?;
        require!(data.len() > 8, SoldexError::MarketNotActive);
        MarketState::try_deserialize(&mut &data[..])?
    };

    let size        = position.size;
    let entry_price = position.entry_price;
    let collateral  = position.collateral;

    let price_delta: i128 = match position.side {
        PositionSide::Long  => mark_price as i128 - entry_price as i128,
        PositionSide::Short => entry_price as i128 - mark_price as i128,
    };
    let raw_pnl: i128 = (size as i128)
        .checked_mul(price_delta)
        .ok_or(SoldexError::Overflow)?
        .checked_div(1_000_000)
        .ok_or(SoldexError::Overflow)?;

    let current_index: i128 = match position.side {
        PositionSide::Long  => market.cumulative_funding_long,
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

    let total_pnl: i128 = raw_pnl
        .checked_add(funding_pnl)
        .ok_or(SoldexError::Overflow)?;
    let equity: i128 = (collateral as i128)
        .checked_add(total_pnl)
        .ok_or(SoldexError::Overflow)?;

    let notional: i128 = (size as i128)
        .checked_mul(mark_price as i128)
        .ok_or(SoldexError::Overflow)?
        .checked_div(1_000_000)
        .ok_or(SoldexError::Overflow)?;
    let maintenance: i128 = notional
        .checked_mul(market.maintenance_margin_bps as i128)
        .ok_or(SoldexError::Overflow)?
        .checked_div(10_000)
        .ok_or(SoldexError::Overflow)?;

    require!(equity < maintenance, SoldexError::NotLiquidatable);

    match position.side {
        PositionSide::Long => {
            market.long_open_interest = market
                .long_open_interest
                .saturating_sub(size);
        }
        PositionSide::Short => {
            market.short_open_interest = market
                .short_open_interest
                .saturating_sub(size);
        }
    }
    position.size           = 0;
    position.collateral     = 0;
    position.is_open        = false;
    position.last_update_ts = Clock::get()?.unix_timestamp;

    {
        let mut pos_data = ctx.accounts.position.try_borrow_mut_data()?;
        position.try_serialize(&mut &mut pos_data[..])?;
    }
    {
        let mut mkt_data = ctx.accounts.market.try_borrow_mut_data()?;
        market.try_serialize(&mut &mut mkt_data[..])?;
    }

    emit!(PositionLiquidated {
        owner:      position.owner,
        liquidator: ctx.accounts.liquidator.key(),
        market_id:  market.market_id,
        mark_price,
    });

    Ok(())
}