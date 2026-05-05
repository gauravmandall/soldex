use anchor_lang::prelude::*;
use crate::errors::SoldexError;
use crate::state::{Position, PositionSide};
use crate::ctx_accounts::ClosePositionEr;

pub fn close_position_er_handler(
    ctx: Context<ClosePositionEr>,
    mark_price: u64,
    _market_id: [u8; 16],
    _nonce: u8,
) -> Result<()> {
    let mut position = {
        let data = ctx.accounts.position.try_borrow_data()?;
        require!(data.len() > 8, SoldexError::PositionNotFound);
        Position::try_deserialize(&mut &data[..])?
    };

    require!(position.is_open, SoldexError::PositionNotOpen);
    require!(position.is_delegated, SoldexError::NotDelegated);
    require!(
        position.owner == ctx.accounts.owner.key(),
        SoldexError::InvalidSeeds
    );
    require!(mark_price > 0, SoldexError::InvalidOraclePrice);

    let size = position.size as i64;
    let entry = position.entry_price as i64;
    let mark = mark_price as i64;

    let raw_pnl: i64 = match position.side {
        PositionSide::Long  => (mark - entry) * size / 1_000_000,
        PositionSide::Short => (entry - mark) * size / 1_000_000,
    };

    position.is_open         = false;
    position.size            = 0;
    position.collateral      = 0;
    position.last_update_ts  = Clock::get()?.unix_timestamp;

    let mut data = ctx.accounts.position.try_borrow_mut_data()?;
    position.try_serialize(&mut &mut data[..])?;

    Ok(())
}
