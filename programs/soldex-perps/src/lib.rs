use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::ephemeral;

declare_id!("7bWbam1aYjH42WLWJWWRM84Y2WXoknUwjbb6ZpATTM6k");

pub mod ctx_accounts;
pub mod errors;
pub mod instructions;
pub mod price;
pub mod state;

pub use ctx_accounts::*;

#[ephemeral]
#[program]
pub mod soldex_perps {
    use super::*;

    // ── Market admin ──────────────────────────────────────────────────────────

    pub fn initialize_market(
        ctx: Context<InitializeMarket>,
        params: InitializeMarketParams,
    ) -> Result<()> {
        instructions::initialize_market::handler(ctx, params)
    }

    pub fn update_market(ctx: Context<UpdateMarket>, params: UpdateMarketParams) -> Result<()> {
        instructions::update_market::handler(ctx, params)
    }

    // ── Collateral ────────────────────────────────────────────────────────────

    pub fn deposit_collateral(ctx: Context<DepositCollateral>, amount: u64) -> Result<()> {
        instructions::deposit_collateral::handler(ctx, amount)
    }

    pub fn withdraw_collateral(ctx: Context<WithdrawCollateral>, amount: u64) -> Result<()> {
        instructions::withdraw_collateral::handler(ctx, amount)
    }

    // ── Trader instructions ───────────────────────────────────────────────────

    pub fn open_position(ctx: Context<OpenPosition>, params: OpenPositionParams) -> Result<()> {
        instructions::open_position::handler(ctx, params)
    }

    pub fn close_position(ctx: Context<ClosePosition>, params: ClosePositionParams) -> Result<()> {
        instructions::close_position::handler(ctx, params)
    }

    pub fn close_position_er(
        ctx: Context<ClosePositionEr>,
        mark_price: u64,
        market_id: [u8; 16],
        nonce: u8,
    ) -> Result<()> {
        instructions::close_position_er::close_position_er_handler(
            ctx, mark_price, market_id, nonce,
        )
    }

    pub fn liquidate(ctx: Context<Liquidate>) -> Result<()> {
        instructions::liquidate::handler(ctx)
    }

    pub fn settle_funding(ctx: Context<SettleFunding>) -> Result<()> {
        instructions::settle_funding::handler(ctx)
    }

    // ── MagicBlock ER ─────────────────────────────────────────────────────────

    pub fn delegate_position(
        ctx: Context<DelegatePosition>,
        market_id: [u8; 16],
        nonce: u8,
        engine_pubkey: Pubkey,
    ) -> Result<()> {
        instructions::delegate_position::delegate_position_handler(
            ctx,
            market_id,
            nonce,
            engine_pubkey,
        )
    }

    pub fn undelegate_position(
        ctx: Context<UndelegatePosition>,
        market_id: [u8; 16],
        nonce: u8,
    ) -> Result<()> {
        instructions::undelegate_position::undelegate_position_handler(ctx, market_id, nonce)
    }
}
