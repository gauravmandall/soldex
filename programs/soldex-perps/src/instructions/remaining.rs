use crate::{errors::SoldexError, state::*, ctx_accounts::*};
use anchor_lang::prelude::*;

// ── update_market ─────────────────────────────────────────────────────────────

pub fn update_market_handler(ctx: Context<UpdateMarket>, params: UpdateMarketParams) -> Result<()> {
    let m = &mut ctx.accounts.market;
    if let Some(v) = params.is_active {
        m.is_active = v;
    }
    if let Some(v) = params.max_leverage_bps {
        m.max_leverage_bps = v;
    }
    if let Some(v) = params.maker_fee_bps {
        m.maker_fee_bps = v;
    }
    if let Some(v) = params.taker_fee_bps {
        m.taker_fee_bps = v;
    }
    Ok(())
}

// ── deposit_collateral ────────────────────────────────────────────────────────

pub fn deposit_collateral_handler(ctx: Context<DepositCollateral>, amount: u64) -> Result<()> {
    // Transfer USDC from user → vault
    anchor_spl::token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::Transfer {
                from: ctx.accounts.user_token_account.to_account_info(),
                to:   ctx.accounts.vault.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
            },
        ),
        amount,
    )?;

    let margin = &mut ctx.accounts.margin;

    // Initialize margin account fields on first deposit only.
    // init_if_needed zeroes the account; Pubkey::default() == zero pubkey.
    if margin.owner == Pubkey::default() {
        margin.owner     = ctx.accounts.owner.key();
        margin.market_id = ctx.accounts.market.market_id;
        margin.bump      = ctx.bumps.margin;
    }

    margin.collateral = margin
        .collateral
        .checked_add(amount)
        .ok_or(SoldexError::Overflow)?;

    Ok(())
}

// ── withdraw_collateral ───────────────────────────────────────────────────────

pub fn withdraw_collateral_handler(ctx: Context<WithdrawCollateral>, amount: u64) -> Result<()> {
    let margin = &mut ctx.accounts.margin;
    require!(
        margin.collateral >= amount,
        SoldexError::InsufficientCollateral
    );
    margin.collateral -= amount;

    // CPI: vault → user
    // Vault is a token account owned by the market PDA — sign with market seeds.
    let market = &ctx.accounts.market;
    let seeds: &[&[u8]] = &[MARKET_SEED, market.market_id.as_ref(), &[market.bump]];

    anchor_spl::token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::Transfer {
                from:      ctx.accounts.vault.to_account_info(),
                to:        ctx.accounts.user_token_account.to_account_info(),
                authority: ctx.accounts.market.to_account_info(),
            },
            &[seeds],
        ),
        amount,
    )?;

    Ok(())
}

// ── liquidate ─────────────────────────────────────────────────────────────────

pub fn liquidate_handler(ctx: Context<Liquidate>) -> Result<()> {
    let market   = &mut ctx.accounts.market;
    let margin   = &mut ctx.accounts.margin;
    let position = &mut ctx.accounts.position;

    // TODO: replace with Pyth oracle CPI
    let mark_price: u64 = 100_000_000;

    // Check if position is under maintenance margin
    let pnl        = position.unrealized_pnl(mark_price);
    let equity     = (position.collateral as i64) + pnl;
    let notional   = position.size.saturating_mul(mark_price) / 1_000_000;
    let maintenance = (notional as i64) * (market.maintenance_margin_bps as i64) / 10_000;

    require!(equity < maintenance, SoldexError::NotLiquidatable);

    // Reduce open interest
    match position.side {
        PositionSide::Long => {
            market.long_open_interest =
                market.long_open_interest.saturating_sub(position.size);
        }
        PositionSide::Short => {
            market.short_open_interest =
                market.short_open_interest.saturating_sub(position.size);
        }
    }

    // Wipe position (simplified — liquidator reward & penalty not yet implemented)
    position.is_open   = false;
    position.size      = 0;
    position.collateral = 0;
    margin.collateral  = 0;

    emit!(PositionLiquidated {
        owner:      position.owner,
        liquidator: ctx.accounts.liquidator.key(),
        market_id:  position.market_id,
        mark_price,
    });

    Ok(())
}

// ── settle_funding ────────────────────────────────────────────────────────────

pub fn settle_funding_handler(ctx: Context<SettleFunding>) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let clock  = Clock::get()?;

    require!(
        clock.unix_timestamp >= market.last_funding_ts + FUNDING_INTERVAL_SECS,
        SoldexError::FundingAlreadySettled
    );

    let oi_diff = (market.long_open_interest as i128) - (market.short_open_interest as i128);

    // base rate = 100 bps (0.01%) per interval + OI skew component
    let funding_rate: i128 = 100 + oi_diff / 1_000_000;

    market.cumulative_funding_long  = market.cumulative_funding_long.saturating_add(funding_rate);
    market.cumulative_funding_short = market.cumulative_funding_short.saturating_sub(funding_rate);
    market.last_funding_ts          = clock.unix_timestamp;

    emit!(FundingSettled {
        market_id:    market.market_id,
        funding_rate,
        timestamp:    clock.unix_timestamp,
    });

    Ok(())
}

// ── close_position ────────────────────────────────────────────────────────────

pub fn close_position_handler(ctx: Context<ClosePosition>) -> Result<()> {
    let position = &mut ctx.accounts.position;
    let market   = &mut ctx.accounts.market;
    let margin   = &mut ctx.accounts.margin;

    // TODO: replace with Pyth oracle CPI
    let mark_price: u64 = 100_000_000;

    // Settle PnL into margin collateral
    let pnl = position.unrealized_pnl(mark_price);
    let new_collateral = (margin.collateral as i64)
        .checked_add(pnl)
        .ok_or(SoldexError::Overflow)?;
    require!(new_collateral >= 0, SoldexError::InsufficientCollateral);
    margin.collateral = new_collateral as u64;

    // Reduce open interest
    match position.side {
        PositionSide::Long => {
            market.long_open_interest =
                market.long_open_interest.saturating_sub(position.size);
        }
        PositionSide::Short => {
            market.short_open_interest =
                market.short_open_interest.saturating_sub(position.size);
        }
    }

    // Close the position
    position.is_open    = false;
    position.size       = 0;
    position.collateral = 0;

    emit!(PositionClosed {
        owner:      position.owner,
        market_id:  position.market_id,
        pnl,
        mark_price,
    });

    Ok(())
}