use crate::errors::SoldexError;
use crate::state::{Position, POSITION_SEED};
use crate::DelegatePosition;
use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::access_control::instructions::{
    CreatePermissionCpi, CreatePermissionCpiAccounts, CreatePermissionInstructionArgs,
};
use ephemeral_rollups_sdk::access_control::structs::{
    Member, MembersArgs, Permission, AUTHORITY_FLAG, TX_BALANCES_FLAG,
};
use ephemeral_rollups_sdk::cpi::DelegateConfig;

/// TEE validator for Private Ephemeral Rollup (Intel TDX, devnet)
const TEE_VALIDATOR: Pubkey = Pubkey::from_str_const("MTEWGuqxUpYZGFJQcp8tLN7x5v9BSeoFHYWQQ3n3xzo");

pub fn delegate_position_handler(
    ctx: Context<DelegatePosition>,
    market_id: [u8; 16],
    nonce: u8,
    engine_pubkey: Pubkey,
) -> Result<()> {
    let clock: Clock = Clock::get()?;

    // ── Guard: account must exist and have data ──────────────────────
    require!(
        ctx.accounts.position.data_len() > 8,
        SoldexError::PositionNotFound
    );

    let mut position = Position::try_deserialize(&mut &ctx.accounts.position.data.borrow()[..])?;

    require!(!position.is_delegated, SoldexError::AlreadyDelegated);
    require!(position.size > 0, SoldexError::PositionNotFound);
    require!(
        position.owner == ctx.accounts.owner.key(),
        SoldexError::InvalidSeeds
    );

    let owner_key = ctx.accounts.owner.key();
    let delegated_at = clock.unix_timestamp;

    position.is_delegated = true;
    position.delegated_at = delegated_at;

    // ── Serialize manually back before CPIs ──────────────────────────
    position.try_serialize(&mut &mut ctx.accounts.position.data.borrow_mut()[..])?;

    emit!(PositionDelegated {
        owner: owner_key,
        market_id,
        delegated_at,
    });

    // ── Create permission: only when permission_program is the real ACL ──
    // On devnet base layer (ER tests), permission_program = SystemProgram → skip.
    // On PER TEE, permission_program = ACLseoPoyC3... → run and set flags.
    let is_per = ctx.accounts.permission_program.key() != anchor_lang::system_program::ID;

    if is_per {
        let (expected_permission_pda, _) = Permission::find_pda(&ctx.accounts.position.key());
        require!(
            ctx.accounts.permission.key() == expected_permission_pda,
            SoldexError::InvalidSeeds
        );

        let nonce_arr = [nonce];
        let bump_arr = [position.bump];
        let position_signer_seeds: &[&[&[u8]]] = &[&[
            POSITION_SEED,
            &market_id,
            ctx.accounts.owner.key.as_ref(),
            &nonce_arr,
            &bump_arr,
        ]];

        // User wallet  → AUTHORITY_FLAG | TX_BALANCES_FLAG
        // Engine key   → AUTHORITY_FLAG only
        // Everyone else → locked out
        let members = vec![
            Member {
                pubkey: ctx.accounts.owner.key(),
                flags: AUTHORITY_FLAG | TX_BALANCES_FLAG,
            },
            Member {
                pubkey: engine_pubkey,
                flags: AUTHORITY_FLAG,
            },
        ];

        CreatePermissionCpi::new(
            &ctx.accounts.permission_program.to_account_info(),
            CreatePermissionCpiAccounts {
                permissioned_account: &ctx.accounts.position.to_account_info(),
                permission: &ctx.accounts.permission.to_account_info(),
                payer: &ctx.accounts.owner.to_account_info(),
                system_program: &ctx.accounts.system_program.to_account_info(),
            },
            CreatePermissionInstructionArgs {
                args: MembersArgs {
                    members: Some(members),
                },
            },
        )
        .invoke_signed(position_signer_seeds)
        .map_err(|_| error!(SoldexError::InvalidSeeds))?;
    }

    // ── Delegate to TEE (PER) or public ER ───────────────────────────
    // DELeGG takes ownership here — must be last CPI
    let nonce_arr = [nonce];
    let seeds: &[&[u8]] = &[
        POSITION_SEED,
        &market_id,
        ctx.accounts.owner.key.as_ref(),
        &nonce_arr,
    ];
    ctx.accounts
        .delegate_position(
            &ctx.accounts.owner,
            seeds,
            DelegateConfig {
                commit_frequency_ms: 10_000,
                validator: Some(TEE_VALIDATOR),
            },
        )
        .map_err(|_| error!(SoldexError::InvalidSeeds))?;

    Ok(())
}

#[event]
pub struct PositionDelegated {
    pub owner: Pubkey,
    pub market_id: [u8; 16],
    pub delegated_at: i64,
}
