use anchor_lang::prelude::*;
use crate::{errors::SoldexError, state::*};

// ── update_market ─────────────────────────────────────────────────────────────

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct UpdateMarketParams {
    pub market_id: [u8; 16],
    pub is_active: Option<bool>,
    pub max_leverage_bps: Option<u64>,
    pub maker_fee_bps: Option<u16>,
    pub taker_fee_bps: Option<u16>,
}

#[derive(Accounts)]
#[instruction(params: UpdateMarketParams)]
pub struct UpdateMarket<'info> {
    #[account(mut, constraint = admin.key() == market.admin @ SoldexError::Unauthorized)]
    pub admin: Signer<'info>,
    #[account(mut, seeds = [b"market", &params.market_id], bump = market.bump)]
    pub market: Account<'info, MarketState>,
}

pub fn handler(ctx: Context<UpdateMarket>, params: UpdateMarketParams) -> Result<()> {
    let m = &mut ctx.accounts.market;
    if let Some(v) = params.is_active { m.is_active = v; }
    if let Some(v) = params.max_leverage_bps { m.max_leverage_bps = v; }
    if let Some(v) = params.maker_fee_bps { m.maker_fee_bps = v; }
    if let Some(v) = params.taker_fee_bps { m.taker_fee_bps = v; }
    Ok(())
}

// ── deposit_collateral ────────────────────────────────────────────────────────

#[derive(Accounts)]
pub struct DepositCollateral<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init_if_needed,
        payer = owner,
        space = MarginAccount::LEN,
        seeds = [b"margin", market.market_id.as_ref(), owner.key().as_ref()],
        bump,
    )]
    pub margin: Account<'info, MarginAccount>,

    pub market: Account<'info, MarketState>,

    #[account(mut, constraint = user_token_account.owner == owner.key())]
    pub user_token_account: Account<'info, anchor_spl::token::TokenAccount>,

    #[account(mut, constraint = vault.key() == market.vault)]
    pub vault: Account<'info, anchor_spl::token::TokenAccount>,

    pub token_program: Program<'info, anchor_spl::token::Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<DepositCollateral>, amount: u64) -> Result<()> {
    // Transfer USDC from user → vault
    anchor_spl::token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::Transfer {
                from: ctx.accounts.user_token_account.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
            },
        ),
        amount,
    )?;

    let margin = &mut ctx.accounts.margin;
    if margin.owner == Pubkey::default() {
        margin.owner = ctx.accounts.owner.key();
        margin.market_id = ctx.accounts.market.market_id;
        margin.bump = ctx.bumps.margin;
    }
    margin.collateral = margin.collateral.checked_add(amount).ok_or(SoldexError::Overflow)?;
    Ok(())
}

// ── withdraw_collateral ───────────────────────────────────────────────────────

#[derive(Accounts)]
pub struct WithdrawCollateral<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [b"margin", market.market_id.as_ref(), owner.key().as_ref()],
        bump = margin.bump,
        constraint = margin.owner == owner.key(),
    )]
    pub margin: Account<'info, MarginAccount>,

    pub market: Account<'info, MarketState>,

    #[account(mut)]
    pub user_token_account: Account<'info, anchor_spl::token::TokenAccount>,

    #[account(mut, constraint = vault.key() == market.vault)]
    pub vault: Account<'info, anchor_spl::token::TokenAccount>,

    pub token_program: Program<'info, anchor_spl::token::Token>,
}

pub fn handler(ctx: Context<WithdrawCollateral>, amount: u64) -> Result<()> {
    let margin = &mut ctx.accounts.margin;
    require!(margin.collateral >= amount, SoldexError::InsufficientCollateral);
    margin.collateral -= amount;

    // CPI: vault → user (vault is a PDA — sign with market seeds)
    let market = &ctx.accounts.market;
    let seeds = &[b"market", market.market_id.as_ref(), &[market.bump]];
    anchor_spl::token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::Transfer {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.user_token_account.to_account_info(),
                authority: ctx.accounts.market.to_account_info(),
            },
            &[seeds],
        ),
        amount,
    )?;
    Ok(())
}

// ── liquidate ─────────────────────────────────────────────────────────────────

#[derive(Accounts)]
pub struct Liquidate<'info> {
    /// Anyone can call liquidate (permissionless keeper)
    pub liquidator: Signer<'info>,

    #[account(mut, seeds = [b"market", position.market_id.as_ref()], bump = market.bump)]
    pub market: Account<'info, MarketState>,

    #[account(mut, seeds = [b"margin", position.market_id.as_ref(), position.owner.as_ref()], bump = margin.bump)]
    pub margin: Account<'info, MarginAccount>,

    #[account(mut, constraint = position.is_open @ SoldexError::PositionNotOpen)]
    pub position: Account<'info, Position>,
}

pub fn handler(ctx: Context<Liquidate>) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let margin = &mut ctx.accounts.margin;
    let position = &mut ctx.accounts.position;

    // Oracle price (placeholder)
    let mark_price: u64 = 100_000_000;

    // Check if liquidatable
    let pnl = position.unrealized_pnl(mark_price);
    let equity = (position.collateral as i64) + pnl;
    let notional = position.size.saturating_mul(mark_price) / 1_000_000;
    let maintenance = (notional as i64) * (market.maintenance_margin_bps as i64) / 10_000;

    require!(equity < maintenance, SoldexError::NotLiquidatable);

    // Close position, penalize, reward liquidator (simplified)
    match position.side {
        PositionSide::Long => {
            market.long_open_interest = market.long_open_interest.saturating_sub(position.size);
        }
        PositionSide::Short => {
            market.short_open_interest = market.short_open_interest.saturating_sub(position.size);
        }
    }

    position.is_open = false;
    position.size = 0;
    position.collateral = 0;

    emit!(PositionLiquidated {
        owner: position.owner,
        liquidator: ctx.accounts.liquidator.key(),
        market_id: position.market_id,
        mark_price,
    });

    Ok(())
}

#[event]
pub struct PositionLiquidated {
    pub owner: Pubkey,
    pub liquidator: Pubkey,
    pub market_id: [u8; 16],
    pub mark_price: u64,
}

// ── settle_funding ────────────────────────────────────────────────────────────

const FUNDING_INTERVAL_SECS: i64 = 8 * 3600; // 8 hours

#[derive(Accounts)]
pub struct SettleFunding<'info> {
    pub keeper: Signer<'info>,
    #[account(mut)]
    pub market: Account<'info, MarketState>,
}

pub fn handler(ctx: Context<SettleFunding>) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let clock = Clock::get()?;

    require!(
        clock.unix_timestamp >= market.last_funding_ts + FUNDING_INTERVAL_SECS,
        SoldexError::FundingAlreadySettled
    );

    // Simple funding: if more longs than shorts, longs pay shorts (and vice versa)
    let oi_diff = (market.long_open_interest as i128) - (market.short_open_interest as i128);
    // funding_rate = 0.01% per 8h base + skew component
    let funding_rate: i128 = 100 + oi_diff / 1_000_000; // scaled to 1e7

    market.cumulative_funding_long = market
        .cumulative_funding_long
        .saturating_add(funding_rate);
    market.cumulative_funding_short = market
        .cumulative_funding_short
        .saturating_sub(funding_rate);
    market.last_funding_ts = clock.unix_timestamp;

    emit!(FundingSettled {
        market_id: market.market_id,
        funding_rate,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct FundingSettled {
    pub market_id: [u8; 16],
    pub funding_rate: i128,
    pub timestamp: i64,
}
