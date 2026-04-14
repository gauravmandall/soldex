use crate::config::EngineConfig;
use dashmap::DashMap;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

pub type OrderId = u64;
pub type MarketId = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Side {
    Bid,
    Ask,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderType {
    Market,
    Limit,
    StopMarket { trigger: Decimal },
    StopLimit { trigger: Decimal, limit: Decimal },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderStatus {
    Open,
    PartiallyFilled { filled_qty: Decimal },
    Filled,
    Cancelled,
    Rejected { reason: String },
}

/// A resting order in the book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: OrderId,
    pub market_id: MarketId,
    pub side: Side,
    pub order_type: OrderType,
    pub price: Decimal,           // limit price (0 for market)
    pub quantity: Decimal,        // original qty
    pub remaining: Decimal,       // unfilled qty
    pub status: OrderStatus,
    pub owner_pubkey: String,     // Solana pubkey (privacy: obfuscated wallet)
    pub timestamp: u64,           // unix ms
    pub client_order_id: String,  // user-provided idempotency key
}

/// Single price level in the book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: Decimal,
    pub total_qty: Decimal,
    pub order_count: usize,
}

/// Snapshot of the orderbook for a market
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderbookSnapshot {
    pub market_id: MarketId,
    pub bids: Vec<PriceLevel>, // sorted desc
    pub asks: Vec<PriceLevel>, // sorted asc
    pub timestamp: u64,
}

/// Trade execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    pub trade_id: u64,
    pub market_id: MarketId,
    pub maker_order_id: OrderId,
    pub taker_order_id: OrderId,
    pub price: Decimal,
    pub quantity: Decimal,
    pub maker_side: Side,
    pub timestamp: u64,
}

/// Per-market order book
struct MarketBook {
    /// Price → (timestamp, order_id) → Order  (bids: highest price first)
    bids: BTreeMap<(ordered_float::NotNan<f64>, u64), Order>,
    asks: BTreeMap<(ordered_float::NotNan<f64>, u64), Order>,
    next_order_id: OrderId,
    next_trade_id: u64,
}

impl MarketBook {
    fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            next_order_id: 1,
            next_trade_id: 1,
        }
    }

    fn snapshot(&self, depth: usize) -> (Vec<PriceLevel>, Vec<PriceLevel>) {
        let mut bid_levels: BTreeMap<ordered_float::NotNan<f64>, (Decimal, usize)> =
            BTreeMap::new();
        let mut ask_levels: BTreeMap<ordered_float::NotNan<f64>, (Decimal, usize)> =
            BTreeMap::new();

        for ((px, _), order) in &self.bids {
            let e = bid_levels.entry(*px).or_default();
            e.0 += order.remaining;
            e.1 += 1;
        }
        for ((px, _), order) in &self.asks {
            let e = ask_levels.entry(*px).or_default();
            e.0 += order.remaining;
            e.1 += 1;
        }

        let bids: Vec<PriceLevel> = bid_levels
            .iter()
            .rev()
            .take(depth)
            .map(|(px, (qty, cnt))| PriceLevel {
                price: Decimal::from_f64_retain(px.into_inner()).unwrap_or_default(),
                total_qty: *qty,
                order_count: *cnt,
            })
            .collect();

        let asks: Vec<PriceLevel> = ask_levels
            .iter()
            .take(depth)
            .map(|(px, (qty, cnt))| PriceLevel {
                price: Decimal::from_f64_retain(px.into_inner()).unwrap_or_default(),
                total_qty: *qty,
                order_count: *cnt,
            })
            .collect();

        (bids, asks)
    }
}

pub struct OrderbookEngine {
    config: Arc<EngineConfig>,
    /// market_id → MarketBook
    books: DashMap<MarketId, RwLock<MarketBook>>,
    /// order_id → market_id for lookups
    order_index: DashMap<OrderId, MarketId>,
}

impl OrderbookEngine {
    pub fn new(config: Arc<EngineConfig>) -> Self {
        Self {
            config,
            books: DashMap::new(),
            order_index: DashMap::new(),
        }
    }

    /// Ensure a market book exists
    pub fn init_market(&self, market_id: &str) {
        self.books
            .entry(market_id.to_string())
            .or_insert_with(|| RwLock::new(MarketBook::new()));
    }

    /// Place an order — returns fills
    pub fn place_order(
        &self,
        market_id: &str,
        side: Side,
        order_type: OrderType,
        price: Decimal,
        quantity: Decimal,
        owner_pubkey: String,
        client_order_id: String,
    ) -> (Order, Vec<Fill>) {
        self.init_market(market_id);
        let book_entry = self.books.get(market_id).unwrap();
        let mut book = book_entry.write().unwrap();

        let order_id = book.next_order_id;
        book.next_order_id += 1;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let mut order = Order {
            id: order_id,
            market_id: market_id.to_string(),
            side: side.clone(),
            order_type: order_type.clone(),
            price,
            quantity,
            remaining: quantity,
            status: OrderStatus::Open,
            owner_pubkey,
            timestamp: now,
            client_order_id,
        };

        let mut fills = Vec::new();

        // Match against opposite side
        match &side {
            Side::Bid => {
                // Match against asks (lowest ask first)
                let ask_keys: Vec<_> = book.asks.keys().cloned().collect();
                for key in ask_keys {
                    if order.remaining.is_zero() {
                        break;
                    }
                    let ask_px = Decimal::from_f64_retain(key.0.into_inner()).unwrap_or_default();
                    // For market orders, always match; for limit, check price
                    let matches = matches!(&order.order_type, OrderType::Market)
                        || order.price >= ask_px;

                    if !matches {
                        break;
                    }

                    if let Some(mut maker) = book.asks.remove(&key) {
                        let fill_qty = order.remaining.min(maker.remaining);
                        let fill_price = maker.price; // maker sets price

                        let fill = Fill {
                            trade_id: book.next_trade_id,
                            market_id: market_id.to_string(),
                            maker_order_id: maker.id,
                            taker_order_id: order.id,
                            price: fill_price,
                            quantity: fill_qty,
                            maker_side: Side::Ask,
                            timestamp: now,
                        };
                        book.next_trade_id += 1;
                        fills.push(fill);

                        order.remaining -= fill_qty;
                        maker.remaining -= fill_qty;

                        if !maker.remaining.is_zero() {
                            let maker_key = (
                                ordered_float::NotNan::new(maker.price.to_f64().unwrap_or(0.0))
                                    .unwrap(),
                                maker.timestamp,
                            );
                            book.asks.insert(maker_key, maker);
                        }
                    }
                }
            }
            Side::Ask => {
                // Match against bids (highest bid first)
                let bid_keys: Vec<_> = book.bids.keys().rev().cloned().collect();
                for key in bid_keys {
                    if order.remaining.is_zero() {
                        break;
                    }
                    let bid_px = Decimal::from_f64_retain(key.0.into_inner()).unwrap_or_default();
                    let matches = matches!(&order.order_type, OrderType::Market)
                        || order.price <= bid_px;

                    if !matches {
                        break;
                    }

                    if let Some(mut maker) = book.bids.remove(&key) {
                        let fill_qty = order.remaining.min(maker.remaining);
                        let fill_price = maker.price;

                        let fill = Fill {
                            trade_id: book.next_trade_id,
                            market_id: market_id.to_string(),
                            maker_order_id: maker.id,
                            taker_order_id: order.id,
                            price: fill_price,
                            quantity: fill_qty,
                            maker_side: Side::Bid,
                            timestamp: now,
                        };
                        book.next_trade_id += 1;
                        fills.push(fill);

                        order.remaining -= fill_qty;
                        maker.remaining -= fill_qty;

                        if !maker.remaining.is_zero() {
                            let maker_key = (
                                ordered_float::NotNan::new(maker.price.to_f64().unwrap_or(0.0))
                                    .unwrap(),
                                maker.timestamp,
                            );
                            book.bids.insert(maker_key, maker);
                        }
                    }
                }
            }
        }

        // Rest unfilled limit orders
        if !order.remaining.is_zero() && matches!(order.order_type, OrderType::Limit) {
            let px = ordered_float::NotNan::new(order.price.to_f64().unwrap_or(0.0)).unwrap();
            let key = (px, order.timestamp);
            match &order.side {
                Side::Bid => {
                    book.bids.insert(key, order.clone());
                }
                Side::Ask => {
                    book.asks.insert(key, order.clone());
                }
            }
        }

        if !fills.is_empty() {
            if order.remaining.is_zero() {
                order.status = OrderStatus::Filled;
            } else {
                order.status = OrderStatus::PartiallyFilled {
                    filled_qty: order.quantity - order.remaining,
                };
            }
        }

        self.order_index.insert(order_id, market_id.to_string());
        (order, fills)
    }

    /// Cancel a resting order
    pub fn cancel_order(&self, order_id: OrderId) -> Option<Order> {
        let market_id = self.order_index.get(&order_id)?.clone();
        let book_entry = self.books.get(&*market_id)?;
        let mut book = book_entry.write().unwrap();

        // Search bids
        let bid_key = book.bids.iter().find_map(|(k, o)| {
            if o.id == order_id { Some(*k) } else { None }
        });
        if let Some(key) = bid_key {
            let mut o = book.bids.remove(&key)?;
            o.status = OrderStatus::Cancelled;
            return Some(o);
        }

        // Search asks
        let ask_key = book.asks.iter().find_map(|(k, o)| {
            if o.id == order_id { Some(*k) } else { None }
        });
        if let Some(key) = ask_key {
            let mut o = book.asks.remove(&key)?;
            o.status = OrderStatus::Cancelled;
            return Some(o);
        }

        None
    }

    /// Get orderbook snapshot for a market
    pub fn get_snapshot(&self, market_id: &str, depth: usize) -> Option<OrderbookSnapshot> {
        let book_entry = self.books.get(market_id)?;
        let book = book_entry.read().unwrap();
        let (bids, asks) = book.snapshot(depth);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        Some(OrderbookSnapshot {
            market_id: market_id.to_string(),
            bids,
            asks,
            timestamp: now,
        })
    }
}

// Shim for f64 → Decimal
trait FromF64 {
    fn from_f64_retain(f: f64) -> Option<Self>
    where
        Self: Sized;
    fn to_f64(&self) -> Option<f64>;
}

impl FromF64 for Decimal {
    fn from_f64_retain(f: f64) -> Option<Self> {
        use std::str::FromStr;
        Decimal::from_str(&f.to_string()).ok()
    }
    fn to_f64(&self) -> Option<f64> {
        use std::str::FromStr;
        f64::from_str(&self.to_string()).ok()
    }
}
