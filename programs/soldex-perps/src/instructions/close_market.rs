use anchor_lang::prelude::*;
use crate::ctx_accounts::CloseMarket;

pub fn handler(ctx: Context<CloseMarket>, _market_id: [u8; 16]) -> Result<()> {
    msg!("Market closed, rent returned to: {}", ctx.accounts.authority.key());
    Ok(())
}