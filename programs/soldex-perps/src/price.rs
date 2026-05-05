use crate::errors::SoldexError;
use anchor_lang::prelude::*;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

pub const MAX_PRICE_AGE_SECS: u64 = 1_000_000; // Make it to 60 seconds in production, but keep it long for tests to avoid flakiness
pub const SOL_USD_FEED_ID: &str =
    "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d";

/// Hardcoded mark price for localnet tests: $150.000000 (scaled 1e6)
pub const TEST_MARK_PRICE: u64 = 150_000_000;

pub fn get_mark_price(price_update: &AccountInfo) -> Result<u64> {
    #[cfg(feature = "test-bpf")]
    {
        let _ = price_update;
        return Ok(TEST_MARK_PRICE);
    }

    #[cfg(not(feature = "test-bpf"))]
    {
        let clock = Clock::get()?;
        let feed_id = parse_feed_id(SOL_USD_FEED_ID)?;
        let price_update = PriceUpdateV2::try_deserialize(&mut &price_update.data.borrow()[..])?;
        let price = price_update
            .get_price_no_older_than(&clock, MAX_PRICE_AGE_SECS, &feed_id)
            .map_err(|_| error!(SoldexError::InvalidOraclePrice))?;
        require!(price.price > 0, SoldexError::InvalidOraclePrice);
        normalize_pyth_price(price.price, price.exponent)
    }
}

fn parse_feed_id(hex: &str) -> Result<[u8; 32]> {
    let hex = hex.trim_start_matches("0x");
    require!(hex.len() == 64, SoldexError::InvalidOraclePrice);
    let mut bytes = [0u8; 32];
    for i in 0..32 {
        bytes[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
            .map_err(|_| error!(SoldexError::InvalidOraclePrice))?;
    }
    Ok(bytes)
}

fn normalize_pyth_price(price: i64, exponent: i32) -> Result<u64> {
    require!(price > 0, SoldexError::InvalidOraclePrice);
    let target_exp: i32 = -6;
    let exp_diff = exponent - target_exp;
    let result = if exp_diff < 0 {
        let divisor = 10i64
            .checked_pow((-exp_diff) as u32)
            .ok_or(SoldexError::Overflow)?;
        price.checked_div(divisor).ok_or(SoldexError::Overflow)?
    } else {
        let multiplier = 10i64
            .checked_pow(exp_diff as u32)
            .ok_or(SoldexError::Overflow)?;
        price.checked_mul(multiplier).ok_or(SoldexError::Overflow)?
    };
    require!(result > 0, SoldexError::InvalidOraclePrice);
    Ok(result as u64)
}
