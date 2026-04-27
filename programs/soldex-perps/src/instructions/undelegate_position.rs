use anchor_lang::prelude::*;
// use ephemeral_rollups_sdk::cpi::undelegate_account;
use crate::{
    errors::SoldexError,
    state::{MarginAccount, MarketState, Position},
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct UndelegatePositionParams {
    /// Same nonce used in open_position and delegate_position.
    /// Needed to reconstruct the Position PDA seeds since nonce is
    /// not stored on the Position account itself.
    pub nonce: u8,
}

#[derive(Accounts)]
#[instruction(params: UndelegatePositionParams)]
pub struct UndelegatePosition<'info> {
    /// The trader who owns this position. Must sign.
    #[account(mut)]
    pub user: Signer<'info>,

    /// Market — not mutated, used for seed derivation only.
    #[account(
        seeds = [b"market", market.market_id.as_ref()],
        bump = market.bump,
    )]
    pub market: Account<'info, MarketState>,

    /// Margin account — not mutated, used for seed derivation only.
    #[account(
        seeds = [b"margin", market.market_id.as_ref(), user.key().as_ref()],
        bump = margin.bump,
    )]
    pub margin: Account<'info, MarginAccount>,

    /// The Position PDA to bring back to base-layer.
    ///
    /// We intentionally do NOT check is_open here.
    /// Reason: the rollup may have closed or liquidated the position
    /// internally. We still need to undelegate to reclaim the account
    /// and settle the final state back to base-layer.
    #[account(
        mut,
        seeds = [
            b"position",
            market.market_id.as_ref(),
            user.key().as_ref(),
            &[params.nonce],
        ],
        bump = position.bump,
        constraint = position.owner == user.key() @ SoldexError::Unauthorized,
    )]
    pub position: Account<'info, Position>,

    // ------------------------------------------------------------------
    // MagicBlock delegation program accounts
    // ------------------------------------------------------------------

    /// CHECK: The MagicBlock delegation program.
    pub delegation_program: AccountInfo<'info>,

    /// CHECK: Buffer holding the rollup's committed final state.
    /// The SDK verifies this address during the CPI.
    #[account(mut)]
    pub delegation_buffer: AccountInfo<'info>,

    /// CHECK: Delegation record — will be closed by the delegation program.
    #[account(mut)]
    pub delegation_record: AccountInfo<'info>,

    /// CHECK: Delegation metadata — will be closed by the delegation program.
    #[account(mut)]
    pub delegation_metadata: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<UndelegatePosition>, params: UndelegatePositionParams) -> Result<()> {
    let market_id = ctx.accounts.market.market_id;
    let owner     = ctx.accounts.user.key();
    let nonce     = params.nonce;
    let bump      = ctx.accounts.position.bump;

    // Seeds must match open_position exactly.
    let signer_seeds: &[&[&[u8]]] = &[&[
        b"position",
        market_id.as_ref(),
        owner.as_ref(),
        &[nonce],
        &[bump],
    ]];

    // TODO: re-enable once MagicBlock SDK dependency issue is resolved
    // undelegate_account(
    //     CpiContext::new_with_signer(
    //         ctx.accounts.delegation_program.to_account_info(),
    //         ephemeral_rollups_sdk::cpi::accounts::UndelegateAccount {
    //             payer:               ctx.accounts.user.to_account_info(),
    //             delegate_account:    ctx.accounts.position.to_account_info(),
    //             delegation_buffer:   ctx.accounts.delegation_buffer.to_account_info(),
    //             delegation_record:   ctx.accounts.delegation_record.to_account_info(),
    //             delegation_metadata: ctx.accounts.delegation_metadata.to_account_info(),
    //             system_program:      ctx.accounts.system_program.to_account_info(),
    //         },
    //         signer_seeds,
    //     ),
    // )?;

    emit!(PositionUndelegated {
        user:      owner,
        market_id,
        nonce,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct PositionUndelegated {
    pub user:      Pubkey,
    pub market_id: [u8; 16],
    pub nonce:     u8,
    pub timestamp: i64,
}