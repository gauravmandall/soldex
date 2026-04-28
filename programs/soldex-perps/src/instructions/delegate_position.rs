use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::cpi::DelegateConfig;
use crate::errors::SoldexError;
use crate::state::POSITION_SEED;
use crate::DelegatePosition;

pub fn delegate_position_handler(ctx: Context<DelegatePosition>, market_id: [u8; 16], nonce: u8) -> Result<()> {
    let clock = Clock::get()?;

    require!(!ctx.accounts.position.is_delegated, SoldexError::AlreadyDelegated);
    require!(ctx.accounts.position.size > 0, SoldexError::PositionNotFound);

    let seeds: &[&[u8]] = &[
        POSITION_SEED,
        ctx.accounts.owner.key.as_ref(),
        &market_id,
        &[nonce],   // ← add nonce
    ];

    ctx.accounts
        .delegate_position(&ctx.accounts.owner, seeds, DelegateConfig::default())
        .map_err(|_| error!(SoldexError::InvalidSeeds))?;

    let position = &mut ctx.accounts.position;
    position.is_delegated = true;
    position.delegated_at = clock.unix_timestamp;

    emit!(PositionDelegated {
        owner: ctx.accounts.owner.key(),
        market_id,
        delegated_at: clock.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct PositionDelegated {
    pub owner: Pubkey,
    pub market_id: [u8; 16],
    pub delegated_at: i64,
}
