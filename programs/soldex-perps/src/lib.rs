use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount, Transfer},
};

declare_id!("REPLACE_WITH_PROGRAM_ID");

pub mod errors;
pub mod instructions;
pub mod state;

use instructions::*;
use state::*;

#[program]
pub mod soldex_perps {
    use super::*;

    // ── Market admin ────────────────────────────────────────────────────────

    /// Initialize a new perps market (admin only)
    pub fn initialize_market(
        ctx: Context<InitializeMarket>,
        params: InitializeMarketParams,
    ) -> Result<()> {
        instructions::initialize_market::handler(ctx, params)
    }

    /// Update market parameters (tick size, max leverage, etc.)
    pub fn update_market(ctx: Context<UpdateMarket>, params: UpdateMarketParams) -> Result<()> {
        instructions::update_market::handler(ctx, params)
    }

    // ── Trader instructions ─────────────────────────────────────────────────

    /// Deposit collateral (USDC) into a user's margin account
    pub fn deposit_collateral(ctx: Context<DepositCollateral>, amount: u64) -> Result<()> {
        instructions::deposit_collateral::handler(ctx, amount)
    }

    /// Withdraw collateral (checks margin requirements)
    pub fn withdraw_collateral(ctx: Context<WithdrawCollateral>, amount: u64) -> Result<()> {
        instructions::withdraw_collateral::handler(ctx, amount)
    }

    /// Open a perpetual position (long or short)
    pub fn open_position(
        ctx: Context<OpenPosition>,
        params: OpenPositionParams,
    ) -> Result<()> {
        instructions::open_position::handler(ctx, params)
    }

    /// Close (fully or partially) an existing position
    pub fn close_position(
        ctx: Context<ClosePosition>,
        params: ClosePositionParams,
    ) -> Result<()> {
        instructions::close_position::handler(ctx, params)
    }

    /// Liquidate an undercollateralized position (permissionless)
    pub fn liquidate(ctx: Context<Liquidate>) -> Result<()> {
        instructions::liquidate::handler(ctx)
    }

    /// Apply funding rate payments (called by keeper every 8h)
    pub fn settle_funding(ctx: Context<SettleFunding>) -> Result<()> {
        instructions::settle_funding::handler(ctx)
    }
}
