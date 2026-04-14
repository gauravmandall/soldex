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
}
