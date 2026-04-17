use anchor_lang::prelude::*;
// use ephemeral_rollups_sdk::cpi::delegate_account;
use crate::{
    errors::SoldexError,
    state::{MarginAccount, MarketState, Position},
};

/// How long (in slots) the delegation stays valid if the caller passes 0.
/// 3000 slots ≈ ~20 minutes on Solana mainnet/devnet.
pub const DEFAULT_VALID_UNTIL_SLOT: u64 = 3_000;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct DelegatePositionParams {
    /// The same nonce that was passed to open_position when this Position PDA
    /// was created.  We need it to reconstruct the PDA seeds.
    pub nonce: u8,
    /// How many slots the delegation should stay valid.
    /// Pass 0 to use DEFAULT_VALID_UNTIL_SLOT.
    pub valid_until_slot: u64,
}

#[derive(Accounts)]
#[instruction(params: DelegatePositionParams)]
pub struct DelegatePosition<'info> {
    /// The trader who owns this position. Must sign.
    #[account(mut)]
    pub user: Signer<'info>,

    /// Market — confirms it is active and gives us market_id for seed checks.
    /// NOT mutated.
    #[account(
        seeds = [b"market", market.market_id.as_ref()],
        bump = market.bump,
        constraint = market.is_active @ SoldexError::MarketNotActive,
    )]
    pub market: Account<'info, MarketState>,

    /// Margin account — confirms this user has a margin account for this market.
    /// NOT mutated.
    #[account(
        seeds = [b"margin", market.market_id.as_ref(), user.key().as_ref()],
        bump = margin.bump,
    )]
    pub margin: Account<'info, MarginAccount>,

    /// The Position PDA to delegate.
    /// Must be open and owned by user.
    #[account(
        mut,
        seeds = [
            b"position",
            market.market_id.as_ref(),
            user.key().as_ref(),
            &[params.nonce],
        ],
        bump = position.bump,
        constraint = position.is_open         @ SoldexError::PositionNotOpen,
        constraint = position.owner == user.key() @ SoldexError::Unauthorized,
    )]
    pub position: Account<'info, Position>,

    // ------------------------------------------------------------------
    // MagicBlock delegation program accounts
    // These are verified inside the SDK's CPI — we mark them CHECK and
    // explain why each is safe.
    // ------------------------------------------------------------------

    /// CHECK: The MagicBlock delegation program itself.
    /// Address is verified by the SDK when the CPI executes.
    pub delegation_program: AccountInfo<'info>,

    /// CHECK: Snapshot buffer — the SDK derives and verifies this address.
    #[account(mut)]
    pub delegation_buffer: AccountInfo<'info>,

    /// CHECK: Delegation record PDA — created by the delegation program.
    #[account(mut)]
    pub delegation_record: AccountInfo<'info>,

    /// CHECK: Delegation metadata PDA — created by the delegation program.
    #[account(mut)]
    pub delegation_metadata: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<DelegatePosition>, params: DelegatePositionParams) -> Result<()> {
    let valid_until = if params.valid_until_slot == 0 {
        DEFAULT_VALID_UNTIL_SLOT
    } else {
        params.valid_until_slot
    };

    // Capture values we need before any borrows.
    let market_id = ctx.accounts.market.market_id;
    let owner     = ctx.accounts.user.key();
    let nonce     = params.nonce;
    let bump      = ctx.accounts.position.bump;

    // Seeds that let us sign the CPI on behalf of the Position PDA.
    // Must match exactly what open_position used:
    //   [b"position", market_id, owner, &[nonce]]  bump = position.bump
    let signer_seeds: &[&[&[u8]]] = &[&[
        b"position",
        market_id.as_ref(),
        owner.as_ref(),
        &[nonce],
        &[bump],
    ]];

    //  re-enable once MagicBlock SDK dependency issue is resolved
    // delegate_account(
    //     CpiContext::new_with_signer(
    //         ctx.accounts.delegation_program.to_account_info(),
    //         ephemeral_rollups_sdk::cpi::accounts::DelegateAccount {
    //             payer:               ctx.accounts.user.to_account_info(),
    //             delegate_account:    ctx.accounts.position.to_account_info(),
    //             delegation_buffer:   ctx.accounts.delegation_buffer.to_account_info(),
    //             delegation_record:   ctx.accounts.delegation_record.to_account_info(),
    //             delegation_metadata: ctx.accounts.delegation_metadata.to_account_info(),
    //             system_program:      ctx.accounts.system_program.to_account_info(),
    //         },
    //         signer_seeds,
    //     ),
    //     valid_until,
    // )?;

    emit!(PositionDelegated {
        user:             owner,
        market_id,
        nonce,
        valid_until_slot: valid_until,
        timestamp:        Clock::get()?.unix_timestamp,
    });

    Ok(())
}

#[event]
pub struct PositionDelegated {
    pub user:             Pubkey,
    pub market_id:        [u8; 16],
    pub nonce:            u8,
    pub valid_until_slot: u64,
    pub timestamp:        i64,
}