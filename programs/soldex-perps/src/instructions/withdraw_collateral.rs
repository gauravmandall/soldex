use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer};
use crate::ctx_accounts::WithdrawCollateral;
use crate::errors::SoldexError;
use crate::ctx_accounts::MARKET_SEED;

pub fn handler(ctx: Context<WithdrawCollateral>, amount: u64) -> Result<()> {
    // ── Guards ────────────────────────────────────────────────────────────────
    require!(amount > 0, SoldexError::InsufficientCollateral);

    let margin = &mut ctx.accounts.margin;

    require!(
        margin.collateral >= amount,
        SoldexError::InsufficientCollateral
    );

    // ── Remaining collateral after withdrawal ─────────────────────────────────
    let remaining = margin.collateral
        .checked_sub(amount)
        .ok_or(SoldexError::Overflow)?;

    // ── Margin safety check ───────────────────────────────────────────────────
    // Remaining collateral must cover initial margin on any open positions.
    // unrealized_pnl on MarginAccount tracks net PnL across all positions.
    // equity = remaining + unrealized_pnl (signed)
    let equity: i64 = (remaining as i64)
        .checked_add(margin.unrealized_pnl)
        .ok_or(SoldexError::Overflow)?;

    require!(equity >= 0, SoldexError::WithdrawalMarginViolation);

    // ── Apply debit ───────────────────────────────────────────────────────────
    margin.collateral = remaining;

    // ── CPI: transfer USDC from vault → user ──────────────────────────────────
    // Vault is owned by the market PDA — sign with market seeds
    let market    = &ctx.accounts.market;
    let market_id = market.market_id;
    let bump      = market.bump;
    let seeds: &[&[u8]] = &[
        MARKET_SEED,
        market_id.as_ref(),
        &[bump],
    ];

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
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