use anchor_lang::prelude::*;
use crate::ctx_accounts::{UpdateMarket, UpdateMarketParams};
use crate::errors::SoldexError;

pub fn handler(ctx: Context<UpdateMarket>, params: UpdateMarketParams) -> Result<()> {
    let market = &mut ctx.accounts.market;

    // ── Validate all params BEFORE applying any changes ───────────────────────
    // This ensures we never apply a partial update if any validation fails

    if let Some(v) = params.max_leverage_bps {
        require!(v > 0, SoldexError::LeverageExceeded);
    }

    if let Some(v) = params.taker_fee_bps {
        require!(v > 0, SoldexError::Unauthorized);
    }

    // maker_fee must not exceed taker_fee — overlay incoming values over current
    let new_maker = params.maker_fee_bps.unwrap_or(market.maker_fee_bps);
    let new_taker = params.taker_fee_bps.unwrap_or(market.taker_fee_bps);
    require!(new_maker <= new_taker, SoldexError::Unauthorized);

    // initial margin must always be strictly greater than maintenance margin
    let new_initial      = params.initial_margin_bps.unwrap_or(market.initial_margin_bps);
    let new_maintenance  = params.maintenance_margin_bps.unwrap_or(market.maintenance_margin_bps);
    require!(new_maintenance > 0,               SoldexError::InsufficientCollateral);
    require!(new_initial > new_maintenance,     SoldexError::InsufficientCollateral);

    // ── Apply updates — only touch fields that are Some ───────────────────────
    if let Some(v) = params.is_active {
        market.is_active = v;
    }
    if let Some(v) = params.max_leverage_bps {
        market.max_leverage_bps = v;
    }
    if let Some(v) = params.maker_fee_bps {
        market.maker_fee_bps = v;
    }
    if let Some(v) = params.taker_fee_bps {
        market.taker_fee_bps = v;
    }
    if let Some(v) = params.initial_margin_bps {
        market.initial_margin_bps = v;
    }
    if let Some(v) = params.maintenance_margin_bps {
        market.maintenance_margin_bps = v;
    }

    Ok(())
}