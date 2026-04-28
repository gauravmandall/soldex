use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::ephem::commit_and_undelegate_accounts;
use crate::errors::SoldexError;
use crate::UndelegatePosition;

pub fn undelegate_position_handler(ctx: Context<UndelegatePosition>, market_id: [u8; 16], _nonce: u8) -> Result<()> {
    let clock = Clock::get()?;

    // ── Guards ───────────────────────────────────────────────────────────────
    require!(ctx.accounts.position.is_delegated, SoldexError::NotDelegated);
    require!(ctx.accounts.position.size == 0, SoldexError::PositionNotFlat);

    // ── Clear delegation state ───────────────────────────────────────────────
    {
        let position = &mut ctx.accounts.position;
        position.is_delegated = false;
        position.delegated_at = 0;
    }

    emit!(PositionUndelegated {
        owner: ctx.accounts.owner.key(),
        market_id,
        final_collateral: ctx.accounts.position.collateral,
        ts: clock.unix_timestamp,
    });

    // ── Commit ER state + unlock PDA back to base layer ──────────────────────
    // Must be LAST — account data is frozen after this CPI
    commit_and_undelegate_accounts(
        &ctx.accounts.owner,
        vec![&ctx.accounts.position.to_account_info()],
        &ctx.accounts.magic_context,
        &ctx.accounts.magic_program,
            None, 

    )?;

    Ok(())
}

#[event]
pub struct PositionUndelegated {
    pub owner: Pubkey,
    pub market_id: [u8; 16],
    pub final_collateral: u64,
    pub ts: i64,
}
