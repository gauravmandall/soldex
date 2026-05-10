use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Transfer};
use crate::ctx_accounts::{CloseVault, MARKET_SEED};

pub fn handler(ctx: Context<CloseVault>, market_id: [u8; 16], market_bump: u8) -> Result<()> {
    let seeds: &[&[u8]] = &[MARKET_SEED, market_id.as_ref(), &[market_bump]];
    let signer = &[seeds];

    let balance = ctx.accounts.vault.amount;
    if balance > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.vault.to_account_info(),
                    to: ctx.accounts.destination.to_account_info(),
                    authority: ctx.accounts.market.to_account_info(),
                },
                signer,
            ),
            balance,
        )?;
    }

    token::close_account(CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        CloseAccount {
            account: ctx.accounts.vault.to_account_info(),
            destination: ctx.accounts.authority.to_account_info(),
            authority: ctx.accounts.market.to_account_info(),
        },
        signer,
    ))?;

    msg!("Vault drained and closed");
    Ok(())
}