// Phase 1 of two-phase open — runs on base layer. Creates position PDA, deducts collateral.

use anchor_lang::prelude::*;
use crate::errors::SoldexError;
use crate::state::{Position, PositionSide, MarketState};
use crate::ctx_accounts::{InitPosition, InitPositionParams};

pub fn handler(ctx: Context<InitPosition>, params: InitPositionParams) -> Result<()> {
    // ── Deserialize market (delegated → AccountInfo) ──────────────────────────
    let market = {
        let data = ctx.accounts.market.try_borrow_data()?;
        require!(data.len() > 8, SoldexError::MarketNotActive);
        MarketState::try_deserialize(&mut &data[..])?
    };

    let margin   = &mut ctx.accounts.margin;
    let position = &mut ctx.accounts.position;

    require!(market.is_active, SoldexError::MarketNotActive);
    require!(
        params.leverage_bps > 0 && params.leverage_bps <= market.max_leverage_bps,
        SoldexError::LeverageExceeded
    );
    require!(params.size >= market.lot_size, SoldexError::BelowMinLotSize);
    require!(
        margin.collateral >= params.collateral,
        SoldexError::InsufficientCollateral
    );

    margin.collateral = margin
        .collateral
        .checked_sub(params.collateral)
        .ok_or(SoldexError::InsufficientCollateral)?;

    let clock = Clock::get()?;
    position.set_inner(Position {
        market_id:           market.market_id,
        owner:               ctx.accounts.user.key(),
        side:                params.side,
        size:                params.size,
        collateral:          params.collateral,
        leverage_bps:        params.leverage_bps,
        entry_price:         0,     // ← set by open_position_er on ER
        entry_funding_index: 0,     // ← set by open_position_er on ER
        opened_at:           clock.unix_timestamp,
        is_open:             false, // ← set to true by open_position_er
        is_delegated:        false,
        delegated_at:        0,
        last_update_ts:      0,
        bump:                ctx.bumps.position,
        _reserved:           [0u8; 32],
    });

    Ok(())
}