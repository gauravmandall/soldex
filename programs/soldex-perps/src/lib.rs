use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

pub mod ctx_accounts;
pub mod errors;
pub mod instructions;
pub mod state;

pub use ctx_accounts::*;

#[program]
pub mod soldex_perps {
    use super::*;

    // ── Market admin ─────────────────────────────────────────────────────────

    pub fn initialize_market(
        ctx: Context<InitializeMarket>,
        params: InitializeMarketParams,
    ) -> Result<()> {
        instructions::initialize_market::handler(ctx, params)
    }

    pub fn update_market(ctx: Context<UpdateMarket>, params: UpdateMarketParams) -> Result<()> {
        instructions::remaining::update_market_handler(ctx, params)
    }

    // ── Trader instructions ──────────────────────────────────────────────────

    pub fn deposit_collateral(ctx: Context<DepositCollateral>, amount: u64) -> Result<()> {
        instructions::remaining::deposit_collateral_handler(ctx, amount)
    }

    pub fn withdraw_collateral(ctx: Context<WithdrawCollateral>, amount: u64) -> Result<()> {
        instructions::remaining::withdraw_collateral_handler(ctx, amount)
    }

    pub fn open_position(ctx: Context<OpenPosition>, params: OpenPositionParams) -> Result<()> {
        instructions::open_position::handler(ctx, params)
    }

    pub fn close_position(ctx: Context<ClosePosition>, params: ClosePositionParams) -> Result<()> {
        instructions::remaining::close_position_handler(ctx)
    }

    pub fn liquidate(ctx: Context<Liquidate>) -> Result<()> {
        instructions::remaining::liquidate_handler(ctx)
    }

    pub fn settle_funding(ctx: Context<SettleFunding>) -> Result<()> {
        instructions::remaining::settle_funding_handler(ctx)
    }

    // ── MagicBlock ER ────────────────────────────────────────────────────────
    pub fn delegate_position(
        ctx: Context<DelegatePosition>,
        market_id: [u8; 16],
        nonce: u8,
    ) -> Result<()> {
        instructions::delegate_position::delegate_position_handler(ctx, market_id, nonce)
    }

    pub fn undelegate_position(
        ctx: Context<UndelegatePosition>,
        market_id: [u8; 16],
        nonce: u8,
    ) -> Result<()> {
        instructions::undelegate_position::undelegate_position_handler(ctx, market_id, nonce)
    }
}
