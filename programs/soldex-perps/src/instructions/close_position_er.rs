// Runs on ER. Closes delegated position and subtracts OI from delegated market.

use anchor_lang::prelude::*;
use crate::errors::SoldexError;
use crate::state::{Position, PositionSide, MarketState};
use crate::ctx_accounts::ClosePositionEr;

pub fn close_position_er_handler(
    ctx: Context<ClosePositionEr>,
    mark_price: u64,
    _market_id: [u8; 16],
    _nonce: u8,
) -> Result<()> {
    require!(mark_price > 0, SoldexError::InvalidOraclePrice);

    let mut position = {
        let data = ctx.accounts.position.try_borrow_data()?;
        require!(data.len() > 8, SoldexError::PositionNotFound);
        Position::try_deserialize(&mut &data[..])?
    };

    require!(position.is_open,      SoldexError::PositionNotOpen);
    require!(position.is_delegated, SoldexError::NotDelegated);
    require!(
        position.owner == ctx.accounts.owner.key(),
        SoldexError::InvalidSeeds
    );

    let mut market = {
        let data = ctx.accounts.market.try_borrow_data()?;
        require!(data.len() > 8, SoldexError::MarketNotActive);
        MarketState::try_deserialize(&mut &data[..])?
    };

    match position.side {
        PositionSide::Long => {
            market.long_open_interest = market
                .long_open_interest
                .saturating_sub(position.size);
        }
        PositionSide::Short => {
            market.short_open_interest = market
                .short_open_interest
                .saturating_sub(position.size);
        }
    }

    position.is_open        = false;
    position.size           = 0;
    position.collateral     = 0;
    position.last_update_ts = Clock::get()?.unix_timestamp;

    {
        let mut pos_data = ctx.accounts.position.try_borrow_mut_data()?;
        position.try_serialize(&mut &mut pos_data[..])?;
    }
    {
        let mut mkt_data = ctx.accounts.market.try_borrow_mut_data()?;
        market.try_serialize(&mut &mut mkt_data[..])?;
    }

    Ok(())
}