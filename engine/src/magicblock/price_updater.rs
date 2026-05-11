//! Pipes TickerUpdate prices from broadcast into delegated position cache.
//! Keeps last_mark_price fresh so the liquidation scanner always has current prices.

use std::sync::Arc;

use tokio::sync::broadcast;
use tracing::{debug, warn};

use crate::magicblock::session_registry::SessionRegistry;
use crate::ws::ServerMessage;

/// Convert a market string like "SOL-USDC" or "SOL-PERP" to a zero-padded [u8; 16].
pub fn market_str_to_id(s: &str) -> [u8; 16] {
    let mut id = [0u8; 16];
    let bytes = s.as_bytes();
    let len = bytes.len().min(16);
    id[..len].copy_from_slice(&bytes[..len]);
    id
}

/// Jupiter feed uses "SOL-USDC", program uses "SOL-PERP".
fn feed_market_to_onchain(feed_market: &str) -> Option<&'static str> {
    match feed_market {
        "SOL-USDC" => Some("SOL-PERP"),
        "BTC-USDC" => Some("BTC-PERP"),
        "ETH-USDC" => Some("ETH-PERP"),
        "JUP-USDC" => Some("JUP-PERP"),
        "SOL-PERP" => Some("SOL-PERP"),
        "BTC-PERP" => Some("BTC-PERP"),
        "ETH-PERP" => Some("ETH-PERP"),
        "JUP-PERP" => Some("JUP-PERP"),
        _ => None,
    }
}

/// Convert f64 price (e.g. 150.25) to u64 scaled by 1e6 (150_250_000).
fn price_to_u64(price: f64) -> u64 {
    (price * 1_000_000.0) as u64
}

/// Update all sessions' position caches on each TickerUpdate. Runs as background task.
pub async fn run_price_updater(
    registry: Arc<SessionRegistry>,
    mut rx: broadcast::Receiver<ServerMessage>,
) {
    loop {
        match rx.recv().await {
            Ok(ServerMessage::TickerUpdate { ticker }) => {
                let onchain_name = match feed_market_to_onchain(&ticker.market_id) {
                    Some(n) => n,
                    None => {
                        debug!("No on-chain mapping for feed market: {}", ticker.market_id);
                        continue;
                    }
                };

                let market_id = market_str_to_id(onchain_name);
                let mark_price = price_to_u64(ticker.price);
                if mark_price == 0 {
                    continue;
                }

                let mut updated = 0u32;
                for (_owner, session) in registry.iter_sessions() {
                    for mut entry in session.delegated.iter_mut() {
                        let pos = entry.value_mut();
                        if pos.market_id == market_id {
                            pos.last_mark_price = mark_price;
                            updated += 1;
                        }
                    }
                }

                if updated > 0 {
                    debug!(
                        market = onchain_name,
                        mark_price,
                        positions_updated = updated,
                        "Price cache updated"
                    );
                }
            }

            Err(broadcast::error::RecvError::Lagged(n)) => {
                warn!("Price updater lagged {n} messages — some price updates missed");
            }

            Err(broadcast::error::RecvError::Closed) => {
                warn!("Broadcast channel closed — price updater shutting down");
                break;
            }

            Ok(_) => {}
        }
    }
}
