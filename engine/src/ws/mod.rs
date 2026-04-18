use crate::{
    orderbook::{Fill, Order, OrderbookSnapshot, OrderType, Side},
    perps::{MarketTicker, PositionSide},
    polymarket::{PolyOpportunity, PolyWsMessage},
    AppState,
};
use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

// ─── Protocol ─────────────────────────────────────────────────────────────────

/// Messages FROM frontend TO engine
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Subscribe to a market's orderbook + trades
    Subscribe { market_id: String },
    Unsubscribe { market_id: String },

    /// Place an order (self-custody: signed_tx is a base58 serialized tx)
    PlaceOrder {
        market_id: String,
        side: String,     // "bid" | "ask"
        order_type: String, // "market" | "limit"
        price: f64,
        quantity: f64,
        signed_tx: Option<String>, // for on-chain settlement
        client_order_id: String,
        owner_pubkey: String,
    },

    /// Cancel a resting order
    CancelOrder { order_id: u64 },

    /// Execute a Polymarket order
    PolymarketOrder {
        market_id: String,   // condition_id
        token_id: String,
        side: String,        // "BUY" | "SELL"
        price: f64,
        size: f64,
        signature: String,   // EIP-712 sig from browser wallet
        signer: String,      // Ethereum address
    },

    /// Build an unsigned Solana tx for perps (engine returns tx bytes, frontend signs)
    BuildPerpsOrder {
        market_id: String,
        side: String,         // "long" | "short"
        size: f64,
        leverage: f64,
        collateral_usdc: f64,
        owner_pubkey: String,
    },

    /// Get orderbook snapshot
    GetSnapshot { market_id: String, depth: usize },

    /// Get Polymarket opportunities
    GetPolyOpportunities,

    /// Ping
    Ping,
}

/// Messages FROM engine TO frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Orderbook snapshot (initial or on-demand)
    Snapshot(OrderbookSnapshot),

    /// Incremental orderbook update after a trade
    OrderUpdate { order: Order },
    FillUpdate { fill: Fill },

    /// Market ticker update (every 2s)
    TickerUpdate { ticker: MarketTicker },

    /// Polymarket opportunities
    PolymarketOpportunities { opportunities: Vec<PolyOpportunity> },

    /// Real-time Polymarket updates from CLOB WS
    PolymarketUpdate { update: PolyWsMessage },

    /// Order acknowledged
    OrderAck {
        client_order_id: String,
        order_id: u64,
        status: String,
    },

    /// Unsigned tx bytes for self-custody perps signing
    UnsignedTx {
        request_id: String,
        tx_base64: String,
        description: String,
    },

    /// Generic error
    Error { code: String, message: String },

    /// Pong
    Pong,
}

// ─── Per-connection handler ───────────────────────────────────────────────────

pub async fn handle_client(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut broadcast_rx = state.broadcast_tx.subscribe();

    info!("WebSocket client connected");

    // Task: forward broadcast messages to this client
    let send_task = tokio::spawn(async move {
        loop {
            match broadcast_rx.recv().await {
                Ok(msg) => {
                    let json = match serde_json::to_string(&msg) {
                        Ok(j) => j,
                        Err(e) => {
                            error!("Serialize error: {e}");
                            continue;
                        }
                    };
                    if sender.send(Message::Text(json)).await.is_err() {
                        break; // client disconnected
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("Client lagged {n} messages");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    // Handle incoming messages from this client
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(client_msg) => handle_client_message(client_msg, &state).await,
                    Err(e) => {
                        debug!("Parse error: {e} — raw: {text}");
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    send_task.abort();
    info!("WebSocket client disconnected");
}

async fn handle_client_message(msg: ClientMessage, state: &AppState) {
    match msg {
        ClientMessage::Ping => {
            let _ = state.broadcast_tx.send(ServerMessage::Pong);
        }

        ClientMessage::GetSnapshot { market_id, depth } => {
            if let Some(snap) = state.orderbook.get_snapshot(&market_id, depth) {
                let _ = state.broadcast_tx.send(ServerMessage::Snapshot(snap));
            }
        }

        ClientMessage::PlaceOrder {
            market_id,
            side,
            order_type,
            price,
            quantity,
            signed_tx,
            client_order_id,
            owner_pubkey,
        } => {
            use crate::orderbook::{OrderType, Side};
            let parsed_side = if side == "bid" { Side::Bid } else { Side::Ask };
            let parsed_type = match order_type.as_str() {
                "market" => OrderType::Market,
                _ => OrderType::Limit,
            };

            let price_decimal = rust_decimal::Decimal::from_f64_retain(price).unwrap_or_default();
            let qty_decimal = rust_decimal::Decimal::from_f64_retain(quantity).unwrap_or_default();

            let (order, fills) = state.orderbook.place_order(
                &market_id,
                parsed_side,
                parsed_type,
                price_decimal,
                qty_decimal,
                owner_pubkey,
                client_order_id.clone(),
            );

            let _ = state.broadcast_tx.send(ServerMessage::OrderAck {
                client_order_id,
                order_id: order.id,
                status: format!("{:?}", order.status),
            });

            for fill in fills {
                let _ = state.broadcast_tx.send(ServerMessage::FillUpdate { fill });
            }

            // TODO: if signed_tx is present, submit to Solana
        }

        ClientMessage::CancelOrder { order_id } => {
            if let Some(cancelled) = state.orderbook.cancel_order(order_id) {
                let _ = state
                    .broadcast_tx
                    .send(ServerMessage::OrderUpdate { order: cancelled });
            }
        }

        ClientMessage::GetPolyOpportunities => {
            match state.polymarket.discover_opportunities().await {
                Ok(opps) => {
                    let _ = state
                        .broadcast_tx
                        .send(ServerMessage::PolymarketOpportunities { opportunities: opps });
                }
                Err(e) => {
                    let _ = state.broadcast_tx.send(ServerMessage::Error {
                        code: "POLYMARKET_ERROR".into(),
                        message: e.to_string(),
                    });
                }
            }
        }

        ClientMessage::PolymarketOrder {
            token_id,
            side,
            price,
            size,
            signature,
            signer,
            ..
        } => {
            match state
                .polymarket
                .execute_order(&token_id, &side, price, size, &signature, &signer)
                .await
            {
                Ok(resp) => {
                    let _ = state.broadcast_tx.send(ServerMessage::OrderAck {
                        client_order_id: resp.order_id.clone(),
                        order_id: 0,
                        status: resp.status,
                    });
                }
                Err(e) => {
                    let _ = state.broadcast_tx.send(ServerMessage::Error {
                        code: "POLY_EXEC_ERROR".into(),
                        message: e.to_string(),
                    });
                }
            }
        }

        ClientMessage::BuildPerpsOrder {
            market_id,
            side,
            size,
            leverage,
            collateral_usdc,
            owner_pubkey,
        } => {
            use crate::perps::PositionSide;
            let pos_side = if side == "long" {
                PositionSide::Long
            } else {
                PositionSide::Short
            };

            let collateral_lamports = (collateral_usdc * 1_000_000.0) as u64; // USDC 6 decimals

            match state
                .perps
                .build_open_position_tx(
                    &owner_pubkey,
                    &market_id,
                    pos_side,
                    size,
                    leverage,
                    collateral_lamports,
                )
                .await
            {
                Ok(tx_bytes) => {
                    use base64::{engine::general_purpose, Engine as _};
                    let tx_base64 = general_purpose::STANDARD.encode(&tx_bytes);
                    let _ = state.broadcast_tx.send(ServerMessage::UnsignedTx {
                        request_id: uuid_v4(),
                        tx_base64,
                        description: format!("Open {side} {size} {market_id} x{leverage}"),
                    });
                }
                Err(e) => {
                    let _ = state.broadcast_tx.send(ServerMessage::Error {
                        code: "TX_BUILD_ERROR".into(),
                        message: e.to_string(),
                    });
                }
            }
        }

        _ => {}
    }
}

fn uuid_v4() -> String {
    uuid::Uuid::new_v4().to_string()
}

// shim needed from orderbook mod
use rust_decimal::Decimal;
trait FromF64Retain {
    fn from_f64_retain(f: f64) -> Option<Self>
    where
        Self: Sized;
}
impl FromF64Retain for Decimal {
    fn from_f64_retain(f: f64) -> Option<Self> {
        use std::str::FromStr;
        Decimal::from_str(&f.to_string()).ok()
    }
}
