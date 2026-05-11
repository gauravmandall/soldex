use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer};
use crate::ctx_accounts::DepositCollateral;
use crate::errors::SoldexError;

pub fn handler(ctx: Context<DepositCollateral>, market_id: [u8; 16], amount: u64) -> Result<()> {
    require!(amount > 0, SoldexError::InsufficientCollateral);

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

    let margin: &mut Account<'_, crate::state::MarginAccount> = &mut ctx.accounts.margin;
    if margin.owner == Pubkey::default() {
        margin.owner     = ctx.accounts.owner.key();
        margin.market_id = market_id;  // use arg, not market account
        margin.bump      = ctx.bumps.margin;
    }

    margin.collateral = margin.collateral
        .checked_add(amount)
        .ok_or(SoldexError::Overflow)?;

    Ok(())
}