use anchor_lang::prelude::*;

pub const POSITION_SEED: &[u8] = b"position";
pub const FUNDING_INTERVAL_SECS: i64 = 8 * 3600;
/// Max open interest per side (scaled in lots) — prevents u64 overflow
pub const MAX_OI_PER_SIDE: u64 = 1_000_000_000_000; // 1 trillion lots

// ─── Market State ─────────────────────────────────────────────────────────────

/// Global market state PDA: seeds = [b"market", market_id]
#[account]
// #[derive(Default)]
pub struct MarketState {
    /// e.g. "SOL-PERP" as bytes[16]
    pub market_id: [u8; 16],
    /// Admin who can update params
    pub admin: Pubkey,
    /// USDC mint
    pub quote_mint: Pubkey,
    /// Market vault holding all collateral
    pub vault: Pubkey,
    /// Pyth price feed account
    pub price_feed: Pubkey,
    /// Tick size in basis points (e.g., 100 = $0.01)
    pub tick_size_bps: u64,
    /// Min position size (lot size) in base units
    pub lot_size: u64,
    /// Max leverage allowed (e.g., 2000 = 20x)
    pub max_leverage_bps: u64,
    /// Maker fee in basis points (e.g., 2 = 0.02%)
    pub maker_fee_bps: u16,
    /// Taker fee in basis points (e.g., 5 = 0.05%)
    pub taker_fee_bps: u16,
    /// Initial margin rate in basis points (e.g., 500 = 5%)
    pub initial_margin_bps: u16,
    /// Maintenance margin rate in basis points (e.g., 250 = 2.5%)
    pub maintenance_margin_bps: u16,
    /// Total long open interest in base lots
    pub long_open_interest: u64,
    /// Total short open interest in base lots
    pub short_open_interest: u64,
    /// Cumulative funding index (scaled by 1e9) for longs
    pub cumulative_funding_long: i128,
    /// Cumulative funding index (scaled by 1e9) for shorts
    pub cumulative_funding_short: i128,
    /// Last funding timestamp
    pub last_funding_ts: i64,
    /// Is market accepting new orders
    pub is_active: bool,
    /// PDA bump
    pub bump: u8,
    /// Vault PDA bump seed
    pub vault_bump: u8,
    /// Accumulated protocol fees (scaled 1e6 USDC)
    pub fees_collected: u64,
    /// Reserved for future use
    pub _reserved: [u8; 56],
}

impl Default for MarketState {
    fn default() -> Self {
        Self {
            market_id: [0u8; 16],
            admin: Pubkey::default(),
            quote_mint: Pubkey::default(),
            vault: Pubkey::default(),
            price_feed: Pubkey::default(),
            tick_size_bps: 0,
            lot_size: 0,
            max_leverage_bps: 0,
            maker_fee_bps: 0,
            taker_fee_bps: 0,
            initial_margin_bps: 0,
            maintenance_margin_bps: 0,
            long_open_interest: 0,
            short_open_interest: 0,
            cumulative_funding_long: 0,
            cumulative_funding_short: 0,
            last_funding_ts: 0,
            is_active: false,
            bump: 0,
            vault_bump: 0,
            fees_collected: 0,
            _reserved: [0u8; 56],
        }
    }
}

impl MarketState {
    pub const LEN: usize = 8    // discriminator
        + 16    // market_id
        + 32    // admin
        + 32    // quote_mint
        + 32    // vault
        + 32    // price_feed
        + 8     // tick_size_bps
        + 8     // lot_size
        + 8     // max_leverage_bps
        + 2     // maker_fee_bps
        + 2     // taker_fee_bps
        + 2     // initial_margin_bps
        + 2     // maintenance_margin_bps
        + 8     // long_open_interest
        + 8     // short_open_interest
        + 16    // cumulative_funding_long
        + 16    // cumulative_funding_short
        + 8     // last_funding_ts
        + 1     // is_active
        + 1     // bump
        + 1     // vault_bump
        + 8     // fees_collected
        + 56;   // reserved
}

// ─── User Margin Account ──────────────────────────────────────────────────────

/// Per-user per-market margin account: seeds = [b"margin", market_id, user_pubkey]
#[account]
#[derive(Default)]
pub struct MarginAccount {
    pub market_id: [u8; 16],
    pub owner: Pubkey,
    /// USDC deposited as collateral (6 decimals)
    pub collateral: u64,
    /// Unrealized PnL (can be negative), scaled by 1e6
    pub unrealized_pnl: i64,
    /// Accrued funding payment (negative = user owes)
    pub funding_accrued: i64,
    /// Bump
    pub bump: u8,
}

impl MarginAccount {
    pub const LEN: usize = 8 + 16 + 32 + 8 + 8 + 8 + 1;
}

// ─── Position ─────────────────────────────────────────────────────────────────

/// A single open position: seeds = [b"position", market_id, user_pubkey, position_nonce]
#[account]
pub struct Position {
    pub market_id: [u8; 16],
    pub owner: Pubkey,
    pub side: PositionSide,
    /// Size in base lots (lot_size units)
    pub size: u64,
    /// Entry price in USDC (scaled by 1e6)
    pub entry_price: u64,
    /// Collateral reserved for this position (scaled by 1e6)
    pub collateral: u64,
    /// Leverage in basis points (e.g., 1000 = 10x)
    pub leverage_bps: u64,
    /// Cumulative funding index at entry (for funding settlement)
    pub entry_funding_index: i128,
    /// Position open timestamp
    pub opened_at: i64,
    /// Is this position open
    pub is_open: bool,
    /// Whether position is delegated to MagicBlock ephemeral rollup
    pub is_delegated: bool,
    /// Timestamp when position was delegated
    pub delegated_at: i64,
    /// Last time position state was updated
    pub last_update_ts: i64,
    /// PDA bump seed
    pub bump: u8,
    /// Reserved for future use
    pub _reserved: [u8; 32],
}

impl Position {
    pub const LEN: usize = 8 + 16 + 32 + 1 + 8 + 8 + 8 + 8 + 16 + 8 + 1 + 1 + 8 + 8 + 1 + 32;

    /// Calculate unrealized PnL given current mark price
    pub fn unrealized_pnl(&self, mark_price: u64) -> i64 {
        let size = self.size as i64;
        let entry = self.entry_price as i64;
        let mark = mark_price as i64;

        match self.side {
            PositionSide::Long => (mark - entry) * size / 1_000_000,
            PositionSide::Short => (entry - mark) * size / 1_000_000,
        }
    }

    /// Liquidation price
    pub fn liquidation_price(&self, maintenance_margin_bps: u16) -> u64 {
        let entry = self.entry_price;
        let lev = self.leverage_bps;
        let mm = maintenance_margin_bps as u64;

        match self.side {
            PositionSide::Long => {
                // liq = entry * (1 - 1/leverage + maintenance_margin)
                // Using basis points arithmetic
                entry.saturating_sub(entry * (10_000 / lev).saturating_sub(mm) / 10_000)
            }
            PositionSide::Short => entry + entry * (10_000 / lev).saturating_sub(mm) / 10_000,
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum PositionSide {
    Long,
    Short,
}

impl Default for PositionSide {
    fn default() -> Self {
        PositionSide::Long
    }
}
