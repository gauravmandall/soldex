use crate::errors::SoldexError;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use ephemeral_rollups_sdk::anchor::commit;
use ephemeral_rollups_sdk::anchor::delegate;

// ─── Seed constants ───────────────────────────────────────────────────────────
// Single source of truth — every instruction uses these, never raw strings.

pub const MARKET_SEED: &[u8] = b"market";
pub const VAULT_SEED: &[u8] = b"vault";
pub const MARGIN_SEED: &[u8] = b"margin";
pub const POSITION_SEED: &[u8] = b"position";

// Position PDA seed order (canonical, must match everywhere):
//   [POSITION_SEED, market_id, owner_pubkey, nonce]

// ── Initialize Market ─────────────────────────────────────────────────────────

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InitializeMarketParams {
    pub market_id: [u8; 16],
    pub price_feed: Pubkey,
    pub tick_size_bps: u64,
    pub lot_size: u64,
    pub max_leverage_bps: u64,
    pub maker_fee_bps: u16,
    pub taker_fee_bps: u16,
    pub initial_margin_bps: u16,
    pub maintenance_margin_bps: u16,
}

#[derive(Accounts)]
#[instruction(params: InitializeMarketParams)]
pub struct InitializeMarket<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + std::mem::size_of::<MarketState>() + 64,
        seeds = [MARKET_SEED, params.market_id.as_ref()],
        bump,
    )]
    pub market: Account<'info, MarketState>,

    pub quote_mint: Account<'info, Mint>,

    #[account(
    init,
    payer = admin,
    token::mint = quote_mint,
    token::authority = market,
    seeds = [VAULT_SEED, params.market_id.as_ref()],
    bump,
)]
    pub vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

// ── Update Market ───────────────────────────────────────────────────────────

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct UpdateMarketParams {
    pub market_id: [u8; 16],
    pub is_active: Option<bool>,
    pub max_leverage_bps: Option<u64>,
    pub maker_fee_bps: Option<u16>,
    pub taker_fee_bps: Option<u16>,
    pub initial_margin_bps: Option<u16>, // must always stay > maintenance_margin_bps
    pub maintenance_margin_bps: Option<u16>, // must always stay > 0 and < initial_margin_bps
}

#[derive(Accounts)]
#[instruction(params: UpdateMarketParams)]
pub struct UpdateMarket<'info> {
    #[account(
        mut,
        constraint = admin.key() == market.admin @ SoldexError::Unauthorized,
    )]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [MARKET_SEED, params.market_id.as_ref()],
        bump = market.bump,
    )]
    pub market: Account<'info, MarketState>,
}

// ── Close Market (devnet: permissionless — add upgrade-authority guard before mainnet) ──

#[derive(Accounts)]
#[instruction(market_id: [u8; 16])]
pub struct CloseMarket<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        close = authority,
        seeds = [MARKET_SEED, market_id.as_ref()],
        bump = market.bump,
    )]
    pub market: Account<'info, MarketState>,

    pub system_program: Program<'info, System>,
}

// ── Close Vault ───────────────────────────────────────────────────────────────
#[derive(Accounts)]
#[instruction(market_id: [u8; 16])]
pub struct CloseVault<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK: market PDA may already be closed — only needed as PDA signer for vault CPI
    #[account(
        mut,
        seeds = [MARKET_SEED, market_id.as_ref()],
        bump,
    )]
    pub market: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [VAULT_SEED, market_id.as_ref()],
        bump,
    )]
    pub vault: Account<'info, anchor_spl::token::TokenAccount>,

    /// Receives any leftover tokens from vault before close
    #[account(mut)]
    pub destination: Account<'info, anchor_spl::token::TokenAccount>,

    pub token_program: Program<'info, anchor_spl::token::Token>,
    pub system_program: Program<'info, System>,
}

// ── Deposit Collateral ────────────────────────────────────────────────────────

#[derive(Accounts)]
#[instruction(market_id: [u8; 16], amount: u64)]
pub struct DepositCollateral<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init_if_needed,
        payer = owner,
        space = MarginAccount::LEN,
        seeds = [MARGIN_SEED, market_id.as_ref(), owner.key().as_ref()],
        bump,
    )]
    pub margin: Account<'info, MarginAccount>,

    // market removed — market_id comes from ix arg, no account needed
    #[account(
        mut,
        constraint = user_token_account.owner == owner.key(),
    )]
    pub user_token_account: Account<'info, anchor_spl::token::TokenAccount>,

    #[account(
        mut,
        seeds = [VAULT_SEED, market_id.as_ref()],
        bump,
    )]
    pub vault: Account<'info, anchor_spl::token::TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

// ── Withdraw Collateral ───────────────────────────────────────────────────────

#[derive(Accounts)]
pub struct WithdrawCollateral<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [MARGIN_SEED, market.market_id.as_ref(), owner.key().as_ref()],
        bump = margin.bump,
        constraint = margin.owner == owner.key(),
    )]
    pub margin: Account<'info, MarginAccount>,

    pub market: Account<'info, MarketState>,

    #[account(mut)]
    pub user_token_account: Account<'info, anchor_spl::token::TokenAccount>,

    #[account(
    mut,
    constraint = vault.key() == market.vault,
    constraint = vault.mint == market.quote_mint @ SoldexError::InvalidMint,
    )]
    pub vault: Account<'info, anchor_spl::token::TokenAccount>,

    pub token_program: Program<'info, anchor_spl::token::Token>,
}

// ── Open Position ─────────────────────────────────────────────────────────────

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct OpenPositionParams {
    pub side: PositionSide,
    pub size: u64,
    pub leverage_bps: u64,
    pub collateral: u64,
    pub nonce: u8,
}

#[derive(Accounts)]
#[instruction(params: OpenPositionParams)]
pub struct OpenPosition<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [MARKET_SEED, market.market_id.as_ref()],
        bump = market.bump,
        constraint = market.is_active @ SoldexError::MarketNotActive,
    )]
    pub market: Account<'info, MarketState>,

    #[account(
        mut,
        seeds = [MARGIN_SEED, market.market_id.as_ref(), user.key().as_ref()],
        bump = margin.bump,
    )]
    pub margin: Account<'info, MarginAccount>,

    #[account(
        init,
        payer = user,
        space = Position::LEN,
        // ✓ Canonical seed order: [POSITION_SEED, market_id, owner, nonce]
        seeds = [POSITION_SEED, market.market_id.as_ref(), user.key().as_ref(), &[params.nonce]],
        bump,
    )]
    pub position: Account<'info, Position>,

    /// Pyth price feed account — validated against market.price_feed
    /// CHECK: Pyth price feed — key validated against market.price_feed
    #[account(constraint = price_feed.key() == market.price_feed)]
    pub price_feed: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

// ── Close Position ────────────────────────────────────────────────────────────

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct ClosePositionParams {
    pub nonce: u8,
}

#[derive(Accounts)]
#[instruction(params: ClosePositionParams)]
pub struct ClosePosition<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [MARKET_SEED, market.market_id.as_ref()],
        bump = market.bump,
        constraint = market.is_active @ SoldexError::MarketNotActive,
    )]
    pub market: Account<'info, MarketState>,

    #[account(
        mut,
        seeds = [MARGIN_SEED, market.market_id.as_ref(), user.key().as_ref()],
        bump = margin.bump,
    )]
    pub margin: Account<'info, MarginAccount>,

    #[account(
        mut,
        // ✓ Canonical seed order: [POSITION_SEED, market_id, owner, nonce]
        seeds = [POSITION_SEED, market.market_id.as_ref(), user.key().as_ref(), &[params.nonce]],
        bump = position.bump,
        constraint = position.is_open @ SoldexError::PositionNotOpen,
        constraint = position.owner == user.key() @ SoldexError::InvalidSeeds,
    )]
    pub position: Account<'info, Position>,

    /// Pyth price feed account — validated against market.price_feed
    /// CHECK: Pyth price feed — key validated against market.price_feed
    #[account(constraint = price_feed.key() == market.price_feed)]
    pub price_feed: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

// ── Delegate Position ─────────────────────────────────────────────────────────
// #[delegate] injects: owner_program, delegation_program, system_program
// and for each `del` field: buffer_*, delegation_record_*, delegation_metadata_*

#[delegate]
#[derive(Accounts)]
#[instruction(market_id: [u8; 16], nonce: u8)]
pub struct DelegatePosition<'info> {
    /// CHECK: Written manually before CPI; DELeGG takes ownership after
    #[account(
        mut,
        del,
        seeds = [POSITION_SEED, market_id.as_ref(), owner.key().as_ref(), &[nonce]],
        bump,
    )]
    pub position: AccountInfo<'info>,

    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: ACL permission program (PER) or system program (ER tests).
    /// If system program is passed, permission CPI is skipped in handler.
    pub permission_program: UncheckedAccount<'info>,

    /// CHECK: Permission PDA — only validated and used when permission_program is ACL.
    /// Not marked mut here — CreatePermissionCpi marks it writable internally via CPI.
    pub permission: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

// ── Delegate Market ───────────────────────────────────────────────────────────
/// Delegates MarketState to the MagicBlock ER so funding_tick_er can write to it.
/// Called once per market at engine startup. Admin signs.
#[delegate]
#[derive(Accounts)]
#[instruction(market_id: [u8; 16])]
pub struct DelegateMarket<'info> {
    /// CHECK: MarketState PDA delegated to ER
    #[account(
        mut,
        del,
        seeds = [b"market", market_id.as_ref()],
        bump,
    )]
    pub market: AccountInfo<'info>,

    #[account(mut)]
    pub admin: Signer<'info>,
}

// ── Undelegate Position ───────────────────────────────────────────────────────
#[derive(Accounts)]
#[instruction(market_id: [u8; 16], nonce: u8)]
pub struct UndelegatePosition<'info> {
    /// CHECK: owned by delegation program during undelegation
    #[account(
        mut,
        seeds = [POSITION_SEED, market_id.as_ref(), owner.key().as_ref(), &[nonce]],
        bump,
    )]
    pub position: UncheckedAccount<'info>,

    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: Magic program for ephemeral rollup CPI
    pub magic_program: UncheckedAccount<'info>,

    /// CHECK: Magic context account — validated by delegation program
    #[account(mut)]
    pub magic_context: UncheckedAccount<'info>,
}

// ── Liquidate ─────────────────────────────────────────────────────────────────

#[derive(Accounts)]
pub struct Liquidate<'info> {
    /// Anyone can call liquidate (permissionless keeper)
    pub liquidator: Signer<'info>,

    #[account(
        mut,
        seeds = [MARKET_SEED, position.market_id.as_ref()],
        bump = market.bump,
    )]
    pub market: Account<'info, MarketState>,

    #[account(
        mut,
        seeds = [MARGIN_SEED, position.market_id.as_ref(), position.owner.as_ref()],
        bump = margin.bump,
    )]
    pub margin: Account<'info, MarginAccount>,

    #[account(
        mut,
        constraint = position.is_open @ SoldexError::PositionNotOpen,
    )]
    pub position: Account<'info, Position>,

    /// Pyth price feed account — validated against market.price_feed
    /// CHECK: Pyth price feed — key validated against market.price_feed
    #[account(constraint = price_feed.key() == market.price_feed @ SoldexError::Unauthorized)]
    pub price_feed: UncheckedAccount<'info>,
}

// ── Settle Funding ────────────────────────────────────────────────────────────

#[derive(Accounts)]
pub struct SettleFunding<'info> {
    pub keeper: Signer<'info>,

    #[account(mut)]
    pub market: Account<'info, MarketState>,

    /// Pyth price feed account for mark price
    /// CHECK: Pyth price feed — key validated against market.price_feed
    #[account(constraint = price_feed.key() == market.price_feed)]
    pub price_feed: UncheckedAccount<'info>,
}

// ── Events ────────────────────────────────────────────────────────────────────

#[event]
pub struct PositionLiquidated {
    pub owner: Pubkey,
    pub liquidator: Pubkey,
    pub market_id: [u8; 16],
    pub mark_price: u64,
}

#[event]
pub struct FundingSettled {
    pub market_id: [u8; 16],
    pub funding_rate: i128,
    pub timestamp: i64,
}

#[event]
pub struct PositionClosed {
    pub owner: Pubkey,
    pub market_id: [u8; 16],
    pub pnl: i64,
    pub mark_price: u64,
}

// ── Close Position ER ─────────────────────────────────────────────────────────
#[derive(Accounts)]
#[instruction(mark_price: u64, market_id: [u8; 16], nonce: u8)]
pub struct ClosePositionEr<'info> {
    /// CHECK: delegated position on ER — manually verified in handler
    #[account(
        mut,
        seeds = [POSITION_SEED, market_id.as_ref(), owner.key().as_ref(), &[nonce]],
        bump,
    )]
    pub position: AccountInfo<'info>,

    /// Engine keypair signs on ER (enforced by TEE + ACL).
    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: Delegated MarketState on ER — writable so handler can subtract OI.
    /// Market is delegated alongside positions, so ER owns it and writes succeed.
    #[account(
        mut,
        seeds = [MARKET_SEED, market_id.as_ref()],
        bump,
    )]
    pub market: AccountInfo<'info>,
}

// ── Funding Tick ER ───────────────────────────────────────────────────────────
/// ER variant of SettleFunding. mark_price in 1e6 units — no Pyth on ER.
#[derive(Accounts)]
pub struct FundingTickEr<'info> {
    pub keeper: Signer<'info>,
    /// CHECK: delegated MarketState — owned by delegation program on ER
    #[account(mut)]
    pub market: AccountInfo<'info>,
}

// ── Liquidate ER ──────────────────────────────────────────────────────────────
#[derive(Accounts)]
#[instruction(mark_price: u64, market_id: [u8; 16], nonce: u8)]
pub struct LiquidateEr<'info> {
    /// Permissionless — anyone can liquidate
    pub liquidator: Signer<'info>,

    /// CHECK: delegated position on ER — ownership + fields verified in handler
    #[account(
        mut,
        seeds = [POSITION_SEED, market_id.as_ref(), owner.key().as_ref(), &[nonce]],
        bump,
    )]
    pub position: AccountInfo<'info>,

    /// CHECK: position owner — only for PDA seed; validated against position.owner in handler
    pub owner: UncheckedAccount<'info>,

    /// CHECK: Delegated MarketState on ER — mut for OI update on liquidation.
    #[account(
        mut,
        seeds = [MARKET_SEED, market_id.as_ref()],
        bump,
    )]
    pub market: AccountInfo<'info>,
}

// ── Init Position (base layer — phase 1 of two-phase open) ───────────────────

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InitPositionParams {
    pub market_id: [u8; 16],
    pub side: PositionSide,
    pub size: u64,
    pub leverage_bps: u64,
    pub collateral: u64,
    pub nonce: u8,
}

#[derive(Accounts)]
#[instruction(params: InitPositionParams)]
pub struct InitPosition<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: Delegated MarketState — PDA verified by seeds, fields verified in handler.
    #[account(
        seeds = [MARKET_SEED, params.market_id.as_ref()],
        bump,
    )]
    pub market: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [MARGIN_SEED, params.market_id.as_ref(), user.key().as_ref()],
        bump = margin.bump,
    )]
    pub margin: Account<'info, MarginAccount>,

    #[account(
        init,
        payer = user,
        space = Position::LEN,
        seeds = [POSITION_SEED, params.market_id.as_ref(), user.key().as_ref(), &[params.nonce]],
        bump,
    )]
    pub position: Account<'info, Position>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

// ── Open Position ER (ephemeral rollup — phase 2 of two-phase open) ───────────

#[derive(Accounts)]
#[instruction(mark_price: u64, market_id: [u8; 16], nonce: u8)]
pub struct OpenPositionEr<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: Delegated position on ER — ownership + fields verified in handler.
    #[account(
        mut,
        seeds = [POSITION_SEED, market_id.as_ref(), owner.key().as_ref(), &[nonce]],
        bump,
    )]
    pub position: AccountInfo<'info>,

    /// CHECK: Delegated MarketState on ER — writable because ER owns it after
    /// delegate_market. Fields verified in handler.
    #[account(
        mut,
        seeds = [MARKET_SEED, market_id.as_ref()],
        bump,
    )]
    pub market: AccountInfo<'info>,
}
