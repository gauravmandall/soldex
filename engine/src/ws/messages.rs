/// ws/messages.rs
///
/// Complete WebSocket message protocol including all MagicBlock-related
/// message types for delegate, undelegate, ER updates, and close.
use serde::{Deserialize, Serialize};

// ─── Client → Engine ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMsg {
    // ── Market data ────────────────────────────────────────────────────────
    Subscribe { market_id: String },
    Unsubscribe { market_id: String },
    GetSnapshot { market_id: String },
    Ping,

    // ── Order management ───────────────────────────────────────────────────
    PlaceOrder(PlaceOrderMsg),
    CancelOrder { order_id: String },

    // ── Perps (base layer, user-signed) ────────────────────────────────────
    BuildPerpsOrder(BuildPerpsOrderMsg),

    // ── MagicBlock: delegation lifecycle ───────────────────────────────────
    /// Request an unsigned delegate_position tx for the frontend to sign.
    DelegatePosition(DelegatePositionMsg),
    /// Request an unsigned close_position_er tx (user sign) then undelegate.
    CloseAndUndelegatePosition(CloseUndelegateMsg),
    /// Request status of a delegated position
    GetDelegatedPosition { position_pda: String },

    // ── Polymarket ─────────────────────────────────────────────────────────
    GetPolyOpportunities,
    PolymarketOrder(PolyOrderMsg),
}

#[derive(Debug, Deserialize)]
pub struct PlaceOrderMsg {
    pub market_id: String,
    pub side: Side,
    pub price: u64,
    pub size: u64,
    pub order_type: OrderType,
    pub owner_pubkey: String,
    pub request_id: String,
}

#[derive(Debug, Deserialize)]
pub struct BuildPerpsOrderMsg {
    pub market_id: String,
    pub side: Side,
    pub size: u64,
    pub leverage_bps: u16,
    pub collateral_usdc: u64,
    pub owner_pubkey: String,
    pub request_id: String,
}

#[derive(Debug, Deserialize)]
pub struct DelegatePositionMsg {
    /// The owner's pubkey (base58)
    pub owner_pubkey: String,
    /// The position PDA pubkey (base58)
    pub position_pda: String,
    /// Market id string e.g. "SOL-PERP"
    pub market_id: String,
    /// Optional: pin to a specific ER validator (base58 pubkey)
    pub validator_pubkey: Option<String>,
    pub request_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CloseUndelegateMsg {
    pub owner_pubkey: String,
    pub position_pda: String,
    pub market_id: String,
    /// Exit price from user (will be validated against oracle with slippage tolerance)
    pub exit_price: u64,
    pub request_id: String,
}

#[derive(Debug, Deserialize)]
pub struct PolyOrderMsg {
    pub market_id: String,
    pub side: Side,
    pub price: f64,
    pub size: f64,
    pub owner_pubkey: String,
    pub request_id: String,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Long,
    Short,
    Buy,
    Sell,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderType {
    Market,
    Limit,
}

// ─── Engine → Client ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMsg {
    Pong,
    Error { code: String, message: String },

    // Market data
    Snapshot(SnapshotMsg),
    TickerUpdate(TickerMsg),
    FillUpdate(FillMsg),
    OrderAck(OrderAckMsg),

    // Perps base layer
    UnsignedTx(UnsignedTxMsg),

    // MagicBlock: delegation
    /// Unsigned delegate_position tx — frontend signs and submits to base layer
    UnsignedDelegateTx(UnsignedTxMsg),
    /// Delegation confirmed on base layer — position is now in ER
    DelegationConfirmed(DelegationConfirmedMsg),
    /// Unsigned close_position_er tx — frontend signs and submits to ER
    UnsignedCloseErTx(UnsignedTxMsg),
    /// Unsigned undelegate_position tx — frontend signs and submits to ER
    UnsignedUndelegateTx(UnsignedTxMsg),
    /// Live position update streamed from ER (sent by keeper after each update)
    ErPositionUpdate(ErPositionUpdateMsg),
    /// Position has been returned to base layer
    UndelegationConfirmed(UndelegationConfirmedMsg),
    /// Position was liquidated in ER
    LiquidationAlert(LiquidationAlertMsg),

    // Polymarket
    PolymarketOpportunities(Vec<PolyOpportunity>),
}

#[derive(Debug, Serialize)]
pub struct UnsignedTxMsg {
    pub request_id: String,
    /// Base64-encoded serialised Transaction
    pub tx_base64: String,
    /// Human-readable description for wallet UI
    pub description: String,
    /// Which RPC to submit to: "base" or "er"
    pub submit_to: SubmitTarget,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SubmitTarget {
    Base,
    Er,
}

#[derive(Debug, Serialize)]
pub struct DelegationConfirmedMsg {
    pub owner: String,
    pub position_pda: String,
    pub market_id: String,
    pub er_rpc_url: String, // the ER endpoint the frontend should use for monitoring
    pub delegated_at: i64,
}

#[derive(Debug, Serialize)]
pub struct ErPositionUpdateMsg {
    pub position_pda: String,
    pub market_id: String,
    pub size: u64,
    pub mark_price: u64,
    pub unrealised_pnl: i64,
    pub margin_ratio_bps: u64,
    pub pending_funding: i64,
    /// Milliseconds since epoch — for latency measurement
    pub ts_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct UndelegationConfirmedMsg {
    pub position_pda: String,
    pub market_id: String,
    pub final_collateral: u64,
    pub ts: i64,
}

#[derive(Debug, Serialize)]
pub struct LiquidationAlertMsg {
    pub position_pda: String,
    pub market_id: String,
    pub mark_price: u64,
    pub margin_ratio_bps: u64,
    pub remaining_collateral: u64,
}

#[derive(Debug, Serialize)]
pub struct SnapshotMsg {
    pub market_id: String,
    pub bids: Vec<[u64; 2]>, // [price, size]
    pub asks: Vec<[u64; 2]>,
    pub last_price: u64,
    pub funding_rate_hourly: i64,
    pub open_interest_long: u64,
    pub open_interest_short: u64,
}

#[derive(Debug, Serialize)]
pub struct TickerMsg {
    pub market_id: String,
    pub mark_price: u64,
    pub index_price: u64,
    pub funding_rate_hourly: i64,
    pub open_interest_long: u64,
    pub open_interest_short: u64,
    pub volume_24h: u64,
    pub ts: i64,
}

#[derive(Debug, Serialize)]
pub struct FillMsg {
    pub market_id: String,
    pub price: u64,
    pub size: u64,
    pub side: String,
    pub ts: i64,
}

#[derive(Debug, Serialize)]
pub struct OrderAckMsg {
    pub request_id: String,
    pub order_id: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct PolyOpportunity {
    pub market_id: String,
    pub question: String,
    pub yes_price: f64,
    pub no_price: f64,
    pub volume: f64,
}