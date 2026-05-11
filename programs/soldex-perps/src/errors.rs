use anchor_lang::prelude::*;

#[error_code]
pub enum SoldexError {
    #[msg("Market is not active")]
    MarketNotActive,

    #[msg("Insufficient collateral for this position")]
    InsufficientCollateral,

    #[msg("Leverage exceeds market maximum")]
    LeverageExceeded,

    #[msg("Position size below minimum lot size")]
    BelowMinLotSize,

    #[msg("Position is not open")]
    PositionNotOpen,

    #[msg("Caller is not the position owner")]
    NotPositionOwner,

    #[msg("Position is not eligible for liquidation")]
    NotLiquidatable,

    #[msg("Unauthorized: caller is not market admin")]
    Unauthorized,

    #[msg("Funding rate has already been settled recently")]
    FundingAlreadySettled,

    #[msg("Invalid price from oracle")]
    InvalidOraclePrice,

    #[msg("Arithmetic overflow")]
    Overflow,

    #[msg("Insufficient vault balance")]
    InsufficientVaultBalance,

    #[msg("Withdrawal would violate margin requirements")]
    WithdrawalMarginViolation,

    // ── Added for MagicBlock delegation ──────────────────────────────────────
    #[msg("Position is already delegated to the ephemeral rollup")]
    AlreadyDelegated,

    #[msg("No open position found for this account")]
    PositionNotFound,

    #[msg("PDA seeds do not match — invalid account derivation")]
    InvalidSeeds,

    // ── Added for MagicBlock undelegation ─────────────────────────────────────
    #[msg("Position is not delegated to the ephemeral rollup")]
    NotDelegated,

    #[msg("Position still has open size — close before undelegating")]
    PositionNotFlat,

    #[msg("Token mint does not match market quote mint")]
    InvalidMint,
}
