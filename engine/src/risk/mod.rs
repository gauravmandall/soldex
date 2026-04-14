/// Risk Engine
/// Validates margin requirements, computes liquidations, and
/// enforces position size limits before orders reach the chain.

use serde::{Deserialize, Serialize};

pub struct RiskEngine {
    pub initial_margin_rate: f64,      // 5%
    pub maintenance_margin_rate: f64,  // 2.5%
    pub max_position_usd: f64,
    pub max_leverage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarginCheck {
    pub passed: bool,
    pub required_margin: f64,
    pub available_margin: f64,
    pub leverage: f64,
    pub liquidation_price: f64,
    pub rejection_reason: Option<String>,
}

impl RiskEngine {
    pub fn new() -> Self {
        Self {
            initial_margin_rate: 0.05,
            maintenance_margin_rate: 0.025,
            max_position_usd: 100_000.0,
            max_leverage: 20.0,
        }
    }

    pub fn check_margin(
        &self,
        collateral_usd: f64,
        size: f64,
        price: f64,
        leverage: f64,
        is_long: bool,
    ) -> MarginCheck {
        let notional = size * price;
        let required = notional * self.initial_margin_rate;
        let effective_leverage = notional / collateral_usd;

        let liq_price = if is_long {
            price * (1.0 - 1.0 / leverage + self.maintenance_margin_rate)
        } else {
            price * (1.0 + 1.0 / leverage - self.maintenance_margin_rate)
        };

        let mut reason = None;

        if effective_leverage > self.max_leverage {
            reason = Some(format!(
                "Leverage {:.1}x exceeds maximum {:.1}x",
                effective_leverage, self.max_leverage
            ));
        } else if collateral_usd < required {
            reason = Some(format!(
                "Insufficient margin: need ${:.2}, have ${:.2}",
                required, collateral_usd
            ));
        } else if notional > self.max_position_usd {
            reason = Some(format!(
                "Position size ${:.0} exceeds max ${:.0}",
                notional, self.max_position_usd
            ));
        }

        MarginCheck {
            passed: reason.is_none(),
            required_margin: required,
            available_margin: collateral_usd,
            leverage: effective_leverage,
            liquidation_price: liq_price,
            rejection_reason: reason,
        }
    }

    /// Compute unrealized PnL for a position
    pub fn compute_pnl(
        &self,
        entry_price: f64,
        mark_price: f64,
        size: f64,
        is_long: bool,
    ) -> f64 {
        if is_long {
            (mark_price - entry_price) * size
        } else {
            (entry_price - mark_price) * size
        }
    }

    /// Check if a position should be liquidated
    pub fn should_liquidate(
        &self,
        entry_price: f64,
        mark_price: f64,
        size: f64,
        collateral_usd: f64,
        is_long: bool,
    ) -> bool {
        let pnl = self.compute_pnl(entry_price, mark_price, size, is_long);
        let equity = collateral_usd + pnl;
        let notional = size * mark_price;
        let maintenance_margin = notional * self.maintenance_margin_rate;
        equity < maintenance_margin
    }
}
