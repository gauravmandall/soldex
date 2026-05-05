use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer};
use crate::ctx_accounts::DepositCollateral;
use crate::errors::SoldexError;

pub fn handler(ctx: Context<DepositCollateral>, amount: u64) -> Result<()> {
    // ── Guard ─────────────────────────────────────────────────────────────────
    require!(amount > 0, SoldexError::InsufficientCollateral);

    // ── CPI: transfer USDC from user → vault ──────────────────────────────────
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from:      ctx.accounts.user_token_account.to_account_info(),
                to:        ctx.accounts.vault.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
            },
        ),
        amount,
    )?;

    let margin = &mut ctx.accounts.margin;

    // ── Init margin account fields on first deposit ───────────────────────────
    // init_if_needed zeroes the account — default Pubkey is all zeros
    if margin.owner == Pubkey::default() {
        margin.owner     = ctx.accounts.owner.key();
        margin.market_id = ctx.accounts.market.market_id;
        margin.bump      = ctx.bumps.margin;
    }

    // ── Credit collateral ─────────────────────────────────────────────────────
    margin.collateral = margin.collateral
        .checked_add(amount)
        .ok_or(SoldexError::Overflow)?;

    Ok(())
}