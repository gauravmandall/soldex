use crate::ctx_accounts::{Liquidate, PositionLiquidated};
use crate::errors::SoldexError;
use crate::state::PositionSide;
use crate::price::get_mark_price;
use anchor_lang::prelude::*;


pub fn handler(ctx: Context<Liquidate>) -> Result<()> {
    let mark_price = get_mark_price(&ctx.accounts.price_feed)?;
    let position = &mut ctx.accounts.position;
    let market = &mut ctx.accounts.market;
    let margin = &mut ctx.accounts.margin;

    // ── Guards ────────────────────────────────────────────────────────────────
    require!(!position.is_delegated, SoldexError::AlreadyDelegated); // must undelegate first
    require!(
        position.market_id == market.market_id,
        SoldexError::InvalidSeeds
    );

    let size = position.size;
    let entry_price = position.entry_price;
    let collateral = position.collateral;

    // ── Raw PnL (i128, scaled 1e6 after division) ─────────────────────────────
    let price_delta: i128 = match position.side {
        PositionSide::Long => mark_price as i128 - entry_price as i128,
        PositionSide::Short => entry_price as i128 - mark_price as i128,
    };
    let raw_pnl: i128 = (size as i128)
        .checked_mul(price_delta)
        .ok_or(SoldexError::Overflow)?
        .checked_div(1_000_000) // collapse double 1e6 scaling
        .ok_or(SoldexError::Overflow)?;

    // ── Funding PnL (i128, scaled 1e6 after division) ─────────────────────────
    // Include funding in equity — prevents positions bleeding via funding from escaping liquidation
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
        .checked_div(1_000_000) // collapse double 1e6 scaling
        .ok_or(SoldexError::Overflow)?;

    // ── Total PnL ─────────────────────────────────────────────────────────────
    let total_pnl: i128 = raw_pnl
        .checked_add(funding_pnl)
        .ok_or(SoldexError::Overflow)?;

    // ── Equity = collateral + total_pnl (signed) ──────────────────────────────
    let equity: i128 = (collateral as i128)
        .checked_add(total_pnl)
        .ok_or(SoldexError::Overflow)?;

    // ── Maintenance margin threshold ──────────────────────────────────────────
    // notional = size × mark ÷ 1e6
    let notional: i128 = (size as i128)
        .checked_mul(mark_price as i128)
        .ok_or(SoldexError::Overflow)?
        .checked_div(1_000_000)
        .ok_or(SoldexError::Overflow)?;

    // maintenance = notional × maintenance_margin_bps ÷ 10_000
    let maintenance: i128 = notional
        .checked_mul(market.maintenance_margin_bps as i128)
        .ok_or(SoldexError::Overflow)?
        .checked_div(10_000)
        .ok_or(SoldexError::Overflow)?;

    // ── Liquidation condition ─────────────────────────────────────────────────
    // equity must be below maintenance margin threshold to be liquidatable
    require!(equity < maintenance, SoldexError::NotLiquidatable);

    // Liquidation fee: 10% of remaining collateral goes to protocol
    let liq_fee = collateral / 10;
    market.fees_collected = market
        .fees_collected
        .checked_add(liq_fee)
        .ok_or(SoldexError::Overflow)?;

    // ── Update open interest ──────────────────────────────────────────────────
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

    // ── Wipe position and margin ──────────────────────────────────────────────
    position.size = 0;
    position.collateral = 0;
    margin.collateral = 0; // entire margin wiped — no liquidator reward (simple mode)
    margin.unrealized_pnl = 0;
    position.is_open = false; // set last — no partial state window

    // Use PositionLiquidated event shape defined in ctx_accounts.rs
    emit!(PositionLiquidated {
        owner: position.owner,
        liquidator: ctx.accounts.liquidator.key(),
        market_id: market.market_id,
        mark_price: mark_price,
    });

    Ok(())
}
