use anchor_lang::prelude::*;
use crate::ctx_accounts::DelegateMarket;
use crate::errors::SoldexError;
use ephemeral_rollups_sdk::cpi::DelegateConfig;

/// TEE validator for Private Ephemeral Rollup (Intel TDX, devnet)
const TEE_VALIDATOR: Pubkey = Pubkey::from_str_const("MTEWGuqxUpYZGFJQcp8tLN7x5v9BSeoFHYWQQ3n3xzo");

pub fn handler(ctx: Context<DelegateMarket>, market_id: [u8; 16]) -> Result<()> {
    require!(
        ctx.accounts.market.data_len() > 8,
        SoldexError::InvalidSeeds
    );

    let seeds: &[&[u8]] = &[b"market", &market_id];

    ctx.accounts
        .delegate_market(
            &ctx.accounts.admin,
            seeds,
            DelegateConfig {
                commit_frequency_ms: 10_000,
                validator: Some(TEE_VALIDATOR),
            },
        )
        .map_err(|_| error!(SoldexError::InvalidSeeds))?;

    msg!("Market {:?} delegated to PER", market_id);
    Ok(())
}