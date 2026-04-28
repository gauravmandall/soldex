use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::delegate;
use crate::state::*;
use crate::errors::SoldexError;

// ─── Seed constants ───────────────────────────────────────────────────────────
// Single source of truth — every instruction uses these, never raw strings.

pub const MARKET_SEED:   &[u8] = b"market";
pub const VAULT_SEED:    &[u8] = b"vault";
pub const MARGIN_SEED:   &[u8] = b"margin";
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

    /// CHECK: USDC mint — stored on market state, not validated here
    pub quote_mint: AccountInfo<'info>,

    /// CHECK: vault PDA — holds collateral tokens
    #[account(
        init,
        payer = admin,
        space = 0,
        seeds = [VAULT_SEED, params.market_id.as_ref()],
        bump,
    )]
    pub vault: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

// ── Update Market ─────────────────────────────────────────────────────────────

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct UpdateMarketParams {
    pub market_id: [u8; 16],
    pub is_active: Option<bool>,
    pub max_leverage_bps: Option<u64>,
    pub maker_fee_bps: Option<u16>,
    pub taker_fee_bps: Option<u16>,
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

// ── Deposit Collateral ────────────────────────────────────────────────────────

#[derive(Accounts)]
pub struct DepositCollateral<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init_if_needed,
        payer = owner,
        space = MarginAccount::LEN,
        seeds = [MARGIN_SEED, market.market_id.as_ref(), owner.key().as_ref()],
        bump,
    )]
    pub margin: Account<'info, MarginAccount>,

    pub market: Account<'info, MarketState>,

    #[account(
        mut,
        constraint = user_token_account.owner == owner.key(),
    )]
    pub user_token_account: Account<'info, anchor_spl::token::TokenAccount>,

    #[account(
        mut,
        constraint = vault.key() == market.vault,
    )]
    pub vault: Account<'info, anchor_spl::token::TokenAccount>,

    pub token_program: Program<'info, anchor_spl::token::Token>,
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

    /// CHECK: Pyth price feed — validated against market.price_feed
    #[account(constraint = price_feed.key() == market.price_feed)]
    pub price_feed: AccountInfo<'info>,

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

    /// CHECK: Pyth price feed — validated against market.price_feed
    #[account(constraint = price_feed.key() == market.price_feed)]
    pub price_feed: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

// ── Delegate Position ─────────────────────────────────────────────────────────
// #[delegate] injects: owner_program, delegation_program, system_program
// and for each `del` field: buffer_*, delegation_record_*, delegation_metadata_*

#[delegate]
#[derive(Accounts)]
#[instruction(market_id: [u8; 16], nonce: u8)]
pub struct DelegatePosition<'info> {
    #[account(
        mut,
        del,
        // ✓ Canonical seed order: [POSITION_SEED, market_id, owner, nonce]
        seeds = [POSITION_SEED, market_id.as_ref(), owner.key().as_ref(), &[nonce]],
        bump = position.bump,
        has_one = owner @ SoldexError::InvalidSeeds,
    )]
    pub position: Account<'info, Position>,

    #[account(mut)]
    pub owner: Signer<'info>,
}

// ── Undelegate Position ───────────────────────────────────────────────────────

#[derive(Accounts)]
#[instruction(market_id: [u8; 16], nonce: u8)]
pub struct UndelegatePosition<'info> {
    #[account(
        mut,
        // ✓ Canonical seed order: [POSITION_SEED, market_id, owner, nonce]
        seeds = [POSITION_SEED, market_id.as_ref(), owner.key().as_ref(), &[nonce]],
        bump = position.bump,
        has_one = owner @ SoldexError::InvalidSeeds,
    )]
    pub position: Account<'info, Position>,

    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: MagicBlock magic context PDA
    pub magic_context: AccountInfo<'info>,

    /// CHECK: MagicBlock magic program
    pub magic_program: AccountInfo<'info>,
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
}

// ── Settle Funding ────────────────────────────────────────────────────────────

#[derive(Accounts)]
pub struct SettleFunding<'info> {
    pub keeper: Signer<'info>,

    #[account(mut)]
    pub market: Account<'info, MarketState>,
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