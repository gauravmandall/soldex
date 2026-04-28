use anchor_lang::prelude::*;
use crate::errors::SoldexError;
use crate::state::MarketState;
use crate::InitializeMarket;
use crate::InitializeMarketParams;

pub fn handler(ctx: Context<InitializeMarket>, params: InitializeMarketParams) -> Result<()> {
    let market = &mut ctx.accounts.market;

    require!(params.tick_size_bps > 0, SoldexError::Overflow);
    require!(params.max_leverage_bps > 0, SoldexError::LeverageExceeded);
    require!(params.lot_size > 0, SoldexError::BelowMinLotSize);

    market.market_id = params.market_id;
    market.admin = ctx.accounts.admin.key();
    market.quote_mint = ctx.accounts.quote_mint.key();
    market.vault = ctx.accounts.vault.key();
    market.price_feed = params.price_feed;
    market.tick_size_bps = params.tick_size_bps;
    market.lot_size = params.lot_size;
    market.max_leverage_bps = params.max_leverage_bps;
    market.maker_fee_bps = params.maker_fee_bps;
    market.taker_fee_bps = params.taker_fee_bps;
    market.initial_margin_bps = params.initial_margin_bps;
    market.maintenance_margin_bps = params.maintenance_margin_bps;
    market.long_open_interest = 0;
    market.short_open_interest = 0;
    market.cumulative_funding_long = 0;
    market.cumulative_funding_short = 0;
    market.last_funding_ts = Clock::get()?.unix_timestamp;
    market.is_active = true;
    market.bump = ctx.bumps.market;
    market.vault_bump = ctx.bumps.vault;
    market._reserved = [0u8; 64];

    Ok(())
}
