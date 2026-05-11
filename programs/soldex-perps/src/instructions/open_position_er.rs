// Phase 2 of two-phase open — runs on ER. Sets entry price and updates market OI.
// Requires init_position + delegate_position to have run on base layer first.

use crate::ctx_accounts::OpenPositionEr;
use crate::errors::SoldexError;
use crate::state::{MarketState, Position, PositionSide, MAX_OI_PER_SIDE};
use anchor_lang::prelude::*;

// Re-use the PositionOpened event defined alongside the base-layer instruction.
use crate::instructions::open_position::PositionOpened;

pub fn handler(
    ctx: Context<OpenPositionEr>,
    mark_price: u64,
    _market_id: [u8; 16],
    _nonce: u8,
) -> Result<()> {
    require!(mark_price > 0, SoldexError::InvalidOraclePrice);

    let mut market = {
        let data = ctx.accounts.market.try_borrow_data()?;
        require!(data.len() > 8, SoldexError::MarketNotActive);
        MarketState::try_deserialize(&mut &data[..])?
    };

    require!(market.is_active, SoldexError::MarketNotActive);

    let mut position = {
        let data = ctx.accounts.position.try_borrow_data()?;
        require!(data.len() > 8, SoldexError::PositionNotFound);
        Position::try_deserialize(&mut &data[..])?
    };

    require!(position.is_delegated, SoldexError::NotDelegated);
    require!(!position.is_open, SoldexError::InvalidSeeds);
    require!(
        position.owner == ctx.accounts.owner.key(),
        SoldexError::InvalidSeeds
    );

    let notional = (position.size / 1_000_000)
        .checked_mul(mark_price)
        .ok_or(SoldexError::Overflow)?;
    let required_margin = notional
        .checked_mul(market.initial_margin_bps as u64)
        .ok_or(SoldexError::Overflow)?
        / 10_000;
    require!(
        position.collateral >= required_margin,
        SoldexError::InsufficientCollateral
    );

    match position.side {
        PositionSide::Long => {
            let new_oi = market
                .long_open_interest
                .checked_add(position.size)
                .ok_or(SoldexError::Overflow)?;
            require!(new_oi <= MAX_OI_PER_SIDE, SoldexError::Overflow);
            market.long_open_interest = new_oi;
        }
        PositionSide::Short => {
            let new_oi = market
                .short_open_interest
                .checked_add(position.size)
                .ok_or(SoldexError::Overflow)?;
            require!(new_oi <= MAX_OI_PER_SIDE, SoldexError::Overflow);
            market.short_open_interest = new_oi;
        }
    }

    let now = Clock::get()?.unix_timestamp;
    position.entry_price = mark_price;
    position.entry_funding_index = match position.side {
        PositionSide::Long => market.cumulative_funding_long,
        PositionSide::Short => market.cumulative_funding_short,
    };
    position.is_open = true;
    position.last_update_ts = now;

    {
        let mut pos_data = ctx.accounts.position.try_borrow_mut_data()?;
        position.try_serialize(&mut &mut pos_data[..])?;
    }
    {
        let mut mkt_data = ctx.accounts.market.try_borrow_mut_data()?;
        market.try_serialize(&mut &mut mkt_data[..])?;
    }

    emit!(PositionOpened {
        user: position.owner,
        market_id: market.market_id,
        side: position.side,
        size: position.size,
        entry_price: mark_price,
        collateral: position.collateral,
        leverage_bps: position.leverage_bps,
        timestamp: now,
    });

    Ok(())
}
