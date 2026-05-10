use crate::{
    jupiter_prediction::JupiterOpportunity,
    orderbook::{Fill, Order, OrderType, OrderbookSnapshot, Side},
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
    Subscribe {
        market_id: String,
    },
    Unsubscribe {
        market_id: String,
    },

    /// Place an order (self-custody: signed_tx is a base58 serialized tx)
    PlaceOrder {
        market_id: String,
        side: String,       // "bid" | "ask"
        order_type: String, // "market" | "limit"
        price: f64,
        quantity: f64,
        signed_tx: Option<String>, // for on-chain settlement
        client_order_id: String,
        owner_pubkey: String,
    },

    /// Cancel a resting order
    CancelOrder {
        order_id: u64,
    },

    /// Execute a Polymarket order
    PolymarketOrder {
        market_id: String, // condition_id
        token_id: String,
        side: String, // "BUY" | "SELL"
        price: f64,
        size: f64,
        signature: String, // EIP-712 sig from browser wallet
        signer: String,    // Ethereum address
    },

    /// Build an unsigned Solana tx for perps (engine returns tx bytes, frontend signs)
    BuildPerpsOrder {
        market_id: String,
        side: String, // "long" | "short"
        size: f64,
        leverage: f64,
        collateral_usdc: f64,
        owner_pubkey: String,
    },

    /// Get orderbook snapshot
    GetSnapshot {
        market_id: String,
        depth: usize,
    },

    /// Get Polymarket opportunities
    GetPolyOpportunities,
    /// Get Jupiter Prediction opportunities
    GetJupiterOpportunities,
    /// Place a Jupiter Prediction order
    JupiterPredictionOrder {
        market_id: String,
        outcome_index: usize,
        size_usdc: f64,
        owner_pubkey: String,
    },
    /// Submit a signed transaction
    SubmitTransaction {
        tx_base64: String,
    },
    /// Ping
    Ping,

    /// Delegate position to MagicBlock ER
    DelegatePosition {
        request_id: String,
        owner_pubkey: String,
        position_pda: String,
        market_id: String,
        nonce: u8,
    },

    /// Close position on ER and return to base layer
    CloseAndUndelegatePosition {
        request_id: String,
        owner_pubkey: String,
        position_pda: String,
        market_id: String,
    },

    /// Build unsigned deposit_collateral tx
    DepositCollateral {
        market_id: String,
        amount_usdc: f64,
        owner_pubkey: String,
    },

    /// Sent by client after delegation tx is confirmed on-chain.
    /// Engine provisions a Private ER session for this owner and
    /// registers the position into it.
    ActivateSession {
        request_id: String,
        owner_pubkey: String,
        position_pda: String,
        market_id: String,
        nonce: u8,
        size: u64,
        entry_price: u64,
        is_long: bool,
        collateral: u64,
    },
}

/// Messages FROM engine TO frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Orderbook snapshot (initial or on-demand)
    Snapshot(OrderbookSnapshot),

    /// Incremental orderbook update after a trade
    OrderUpdate {
        order: Order,
    },
    FillUpdate {
        fill: Fill,
    },

    /// Market ticker update (every 2s)
    TickerUpdate {
        ticker: MarketTicker,
    },

    /// Polymarket opportunities
    PolymarketOpportunities {
        opportunities: Vec<PolyOpportunity>,
    },
    /// Jupiter Prediction opportunities
    JupiterPredictionOpportunities {
        opportunities: Vec<JupiterOpportunity>,
    },
    /// Real-time Polymarket updates from CLOB WS
    PolymarketUpdate {
        update: PolyWsMessage,
    },
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
        submit_to: String,
        position_pda: String,
        owner_pubkey: String,
        market_id: String,
        nonce: u8,
    },

    /// Generic error
    Error {
        code: String,
        message: String,
    },

    /// Pong
    Pong,

    /// Unsigned delegate tx — client signs and submits to BASE LAYER
    UnsignedDelegateTx {
        request_id: String,
        tx_base64: String,
        description: String,
        submit_to: String,
        position_pda: String,
        owner_pubkey: String,
        market_id: String,
        nonce: u8,
    },

    /// Unsigned undelegate tx — client signs and submits to ER
    UnsignedUndelegateTx {
        request_id: String,
        tx_base64: String,
        description: String,
        submit_to: String, // always "er"
    },

    /// Engine submitted close_er tx on behalf of client — confirmed on ER
    CloseErSubmitted {
        request_id: String,
        signature: String,
        position_pda: String,
    },
    /// Private ER session is live — engine is tracking this position
    SessionActivated {
        request_id: String,
        owner_pubkey: String,
        position_pda: String,
    },
    SessionActivationFailed {
        request_id: String,
        code: String,
        message: String,
    },
    PositionUpdate {
        position: PositionData,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionData {
    pub market_id: String,
    pub owner: String,
    pub side: String,
    pub size: f64,
    pub entry_price: f64,
    pub mark_price: f64,
    pub collateral: f64,
    pub leverage: f64,
    pub unrealized_pnl: f64,
    pub liquidation_price: f64,
    pub funding_payment: f64,
    pub opened_at: i64,
    pub delegation_status: String,
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
            Message::Text(text) => match serde_json::from_str::<ClientMessage>(&text) {
                Ok(client_msg) => handle_client_message(client_msg, &state).await,
                Err(e) => {
                    debug!("Parse error: {e} — raw: {text}");
                }
            },
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
                        .send(ServerMessage::PolymarketOpportunities {
                            opportunities: opps,
                        });
                }
                Err(e) => {
                    let _ = state.broadcast_tx.send(ServerMessage::Error {
                        code: "POLYMARKET_ERROR".into(),
                        message: e.to_string(),
                    });
                }
            }
        }

        ClientMessage::GetJupiterOpportunities => {
            match state.jupiter_prediction.discover_opportunities().await {
                Ok(opps) => {
                    let _ =
                        state
                            .broadcast_tx
                            .send(ServerMessage::JupiterPredictionOpportunities {
                                opportunities: opps,
                            });
                }
                Err(e) => {
                    let _ = state.broadcast_tx.send(ServerMessage::Error {
                        code: "JUPITER_PRED_ERROR".into(),
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
            let pos_side: PositionSide = if side == "long" {
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
                Ok((tx_bytes, nonce, position_pda_str)) => {
                    use base64::{engine::general_purpose, Engine as _};
                    let tx_base64 = general_purpose::STANDARD.encode(&tx_bytes);
                    let _ = state.broadcast_tx.send(ServerMessage::UnsignedTx {
                        request_id: uuid_v4(),
                        tx_base64,
                        description: format!("Open {side} {size} {market_id} x{leverage}"),
                        submit_to: "base".into(),
                        position_pda: position_pda_str,
                        owner_pubkey: owner_pubkey.clone(),
                        market_id: market_id.clone(),
                        nonce,
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

        ClientMessage::JupiterPredictionOrder {
            market_id,
            outcome_index,
            size_usdc,
            owner_pubkey,
        } => {
            match state
                .jupiter_prediction
                .execute_order(&market_id, outcome_index, size_usdc, &owner_pubkey)
                .await
            {
                Ok(tx_bytes) => {
                    use base64::{engine::general_purpose, Engine as _};
                    let tx_base64 = general_purpose::STANDARD.encode(&tx_bytes);
                    let _ = state.broadcast_tx.send(ServerMessage::UnsignedTx {
                        request_id: format!("jup_{}", uuid_v4()),
                        tx_base64,
                        description: format!(
                            "Jupiter Prediction: Buy Outcome #{} on {}",
                            outcome_index, market_id
                        ),
                        submit_to: "base".into(),
                        position_pda: "".into(),
                        owner_pubkey: owner_pubkey.clone(),
                        market_id: market_id.clone(),
                        nonce: 0,
                    });
                }
                Err(e) => {
                    let _ = state.broadcast_tx.send(ServerMessage::Error {
                        code: "JUP_EXEC_ERROR".into(),
                        message: e.to_string(),
                    });
                }
            }
        }

        ClientMessage::SubmitTransaction { tx_base64 } => {
            use base64::{engine::general_purpose, Engine as _};
            use solana_client::rpc_config::RpcSendTransactionConfig;
            use solana_sdk::transaction::VersionedTransaction;

            let tx_bytes = match general_purpose::STANDARD.decode(&tx_base64) {
                Ok(b) => b,
                Err(e) => {
                    let _ = state.broadcast_tx.send(ServerMessage::Error {
                        code: "INVALID_TX".into(),
                        message: e.to_string(),
                    });
                    return;
                }
            };

            let rpc = state.perps.get_rpc_client();
            let tx: VersionedTransaction = match bincode::deserialize(&tx_bytes) {
                Ok(t) => t,
                Err(e) => {
                    error!("Failed to deserialize transaction: {}", e);
                    return;
                }
            };

            match rpc.send_transaction(&tx).await {
                Ok(sig) => {
                    info!("Submitted transaction: {}", sig);
                    let _ = state.broadcast_tx.send(ServerMessage::OrderAck {
                        client_order_id: sig.to_string(),
                        order_id: 0,
                        status: "submitted".into(),
                    });
                }
                Err(e) => {
                    error!("Failed to submit transaction: {}", e);
                    let _ = state.broadcast_tx.send(ServerMessage::Error {
                        code: "RPC_ERROR".into(),
                        message: e.to_string(),
                    });
                }
            }
        }

        ClientMessage::DepositCollateral {
            market_id,
            amount_usdc,
            owner_pubkey,
        } => {
            let amount_lamports = (amount_usdc * 1_000_000.0) as u64;
            match state
                .perps
                .build_deposit_collateral_tx(&owner_pubkey, &market_id, amount_lamports)
                .await
            {
                Ok(tx_bytes) => {
                    use base64::{engine::general_purpose, Engine as _};
                    let tx_base64 = general_purpose::STANDARD.encode(&tx_bytes);
                    let _ = state.broadcast_tx.send(ServerMessage::UnsignedTx {
                        request_id: uuid_v4(),
                        tx_base64,
                        description: format!(
                            "Deposit {} USDC collateral for {}",
                            amount_usdc, market_id
                        ),
                        submit_to: "deposit".into(),
                        position_pda: "".into(),
                        owner_pubkey: owner_pubkey.clone(),
                        market_id: market_id.clone(),
                        nonce: 0,
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
        ClientMessage::DelegatePosition {
            request_id,
            owner_pubkey,
            position_pda,
            market_id,
            nonce,
        } => {
            let owner: solana_sdk::pubkey::Pubkey = match owner_pubkey.parse() {
                Ok(p) => p,
                Err(e) => {
                    let _ = state.broadcast_tx.send(ServerMessage::Error {
                        code: "INVALID_PUBKEY".into(),
                        message: e.to_string(),
                    });
                    return;
                }
            };
            let position_pda_key: solana_sdk::pubkey::Pubkey = match position_pda.parse() {
                Ok(p) => p,
                Err(e) => {
                    let _ = state.broadcast_tx.send(ServerMessage::Error {
                        code: "INVALID_PUBKEY".into(),
                        message: e.to_string(),
                    });
                    return;
                }
            };
            let onchain_id =
                crate::perps::PerpsEngine::feed_to_onchain(&market_id).unwrap_or(&market_id);
            let mid = market_id_to_bytes(onchain_id);
            let blockhash = state.blockhash_cache.get().await;

            let tx = match state.er_tx_builder.delegate_position_tx(
                &owner,
                &position_pda_key,
                mid,
                nonce,
                blockhash,
            ) {
                Ok(tx) => tx,
                Err(e) => {
                    let _ = state.broadcast_tx.send(ServerMessage::Error {
                        code: "TX_BUILD_ERROR".into(),
                        message: e.to_string(),
                    });
                    return;
                }
            };

            // return unsigned tx — client signs and submits
            let tx_bytes = bincode::serialize(&tx).expect("serialize tx");
            use base64::{engine::general_purpose, Engine as _};
            let tx_base64 = general_purpose::STANDARD.encode(&tx_bytes);
            let _ = state.broadcast_tx.send(ServerMessage::UnsignedDelegateTx {
                request_id,
                tx_base64,
                description: format!("Delegate position {} to MagicBlock ER", position_pda),
                submit_to: "delegate".into(),
                position_pda: position_pda.clone(),
                owner_pubkey: owner_pubkey.clone(),
                market_id: market_id.clone(),
                nonce,
            });
        }

        ClientMessage::ActivateSession {
            request_id,
            owner_pubkey,
            position_pda,
            market_id,
            nonce,
            size,
            entry_price,
            is_long,
            collateral,
        } => {
            use crate::magicblock::session::DelegatedPosition;

            let owner: solana_sdk::pubkey::Pubkey = match owner_pubkey.parse() {
                Ok(p) => p,
                Err(e) => {
                    let _ = state
                        .broadcast_tx
                        .send(ServerMessage::SessionActivationFailed {
                            request_id,
                            code: "INVALID_PUBKEY".into(),
                            message: e.to_string(),
                        });
                    return;
                }
            };
            let position_pda_key: solana_sdk::pubkey::Pubkey = match position_pda.parse() {
                Ok(p) => p,
                Err(e) => {
                    let _ = state
                        .broadcast_tx
                        .send(ServerMessage::SessionActivationFailed {
                            request_id,
                            code: "INVALID_PUBKEY".into(),
                            message: e.to_string(),
                        });
                    return;
                }
            };

            let onchain_id =
                crate::perps::PerpsEngine::feed_to_onchain(&market_id).unwrap_or(&market_id);
            let mid = market_id_to_bytes(onchain_id);

            let session = match state.session_registry.get_or_create(&owner).await {
                Ok(s) => s,
                Err(e) => {
                    let _ = state
                        .broadcast_tx
                        .send(ServerMessage::SessionActivationFailed {
                            request_id,
                            code: "SESSION_INIT_FAILED".into(),
                            message: e.to_string(),
                        });
                    return;
                }
            };

            let pos = DelegatedPosition {
                position_pda: position_pda_key,
                owner,
                market_id: mid,
                nonce,
                size,
                entry_price,
                is_long,
                collateral,
                delegated_at: chrono::Utc::now().timestamp(),
                last_mark_price: 0,
                last_update_at: std::time::Instant::now(),
            };

            session.register_delegation(pos.clone());

            // Compute human-unit display values for the frontend positions table
            let size_f64 = pos.size as f64 / 1_000_000.0;
            let entry_f64 = pos.entry_price as f64 / 1_000_000.0;
            let collateral_f64 = pos.collateral as f64 / 1_000_000.0;
            let notional = size_f64 * entry_f64;
            let leverage_f64 = if collateral_f64 > 0.0 {
                notional / collateral_f64
            } else {
                1.0
            };
            let liq_price = if pos.is_long {
                entry_f64 * (1.0 - collateral_f64 / notional.max(0.000001))
            } else {
                entry_f64 * (1.0 + collateral_f64 / notional.max(0.000001))
            };

            let _ = state.broadcast_tx.send(ServerMessage::PositionUpdate {
                position: PositionData {
                    market_id: market_id.clone(),
                    owner: owner_pubkey.clone(),
                    side: if pos.is_long {
                        "long".into()
                    } else {
                        "short".into()
                    },
                    size: size_f64,
                    entry_price: entry_f64,
                    mark_price: entry_f64,
                    collateral: collateral_f64,
                    leverage: leverage_f64,
                    unrealized_pnl: 0.0,
                    liquidation_price: liq_price,
                    funding_payment: 0.0,
                    opened_at: pos.delegated_at,
                    delegation_status: "TEE_ENCRYPTED".into(),
                },
            });

            let _ = state.broadcast_tx.send(ServerMessage::SessionActivated {
                request_id,
                owner_pubkey,
                position_pda,
            });
        }

        ClientMessage::CloseAndUndelegatePosition {
            request_id,
            owner_pubkey,
            position_pda,
            market_id,
        } => {
            let owner: solana_sdk::pubkey::Pubkey = match owner_pubkey.parse() {
                Ok(p) => p,
                Err(e) => {
                    let _ = state.broadcast_tx.send(ServerMessage::Error {
                        code: "INVALID_PUBKEY".into(),
                        message: e.to_string(),
                    });
                    return;
                }
            };
            let position_pda_key: solana_sdk::pubkey::Pubkey = match position_pda.parse() {
                Ok(p) => p,
                Err(e) => {
                    let _ = state.broadcast_tx.send(ServerMessage::Error {
                        code: "INVALID_PUBKEY".into(),
                        message: e.to_string(),
                    });
                    return;
                }
            };
            let onchain_id =
                crate::perps::PerpsEngine::feed_to_onchain(&market_id).unwrap_or(&market_id);
            let mid = market_id_to_bytes(onchain_id);
            // Resolve the owner's private ER session — error if none exists
            let session = match state.session_registry.get(&owner) {
                Some(s) => s,
                None => {
                    let _ = state.broadcast_tx.send(ServerMessage::Error {
                        code: "NO_SESSION".into(),
                        message: format!("no active session for owner {}", owner_pubkey),
                    });
                    return;
                }
            };

            let (mark_price, nonce, last_update_at) = session
                .delegated
                .get(&position_pda_key)
                .map(|p| (p.last_mark_price, p.nonce, p.last_update_at))
                .unwrap_or((0, 0, std::time::Instant::now()));

            if mark_price == 0 {
                let _ = state.broadcast_tx.send(ServerMessage::Error {
                    code: "NO_MARK_PRICE".into(),
                    message: format!(
                        "position {} not found in ER cache or price is 0",
                        position_pda
                    ),
                });
                return;
            }

            if last_update_at.elapsed() > std::time::Duration::from_secs(30) {
                let _ = state.broadcast_tx.send(ServerMessage::Error {
                    code: "STALE_PRICE".into(),
                    message: format!(
                        "mark price for {} is stale (>30s old) — price pump may be down, refusing close",
                        position_pda
                    ),
                });
                return;
            }

            let er_blockhash = match session.recent_blockhash().await {
                Ok(h) => h,
                Err(e) => {
                    let _ = state.broadcast_tx.send(ServerMessage::Error {
                        code: "BLOCKHASH_ERROR".into(),
                        message: e.to_string(),
                    });
                    return;
                }
            };

            let close_tx = match state.er_tx_builder.close_position_er_tx(
                &position_pda_key,
                mid,
                nonce,
                mark_price,
                er_blockhash,
            ) {
                Ok(tx) => tx,
                Err(e) => {
                    let _ = state.broadcast_tx.send(ServerMessage::Error {
                        code: "TX_BUILD_ERROR".into(),
                        message: e.to_string(),
                    });
                    return;
                }
            };
            let session = session.clone();
            let builder = state.er_tx_builder.clone();
            let broadcast = state.broadcast_tx.clone();
            let registry = state.session_registry.clone();

            let sig_preview = bs58::encode(close_tx.signatures[0]).into_string();
            let _ = state.broadcast_tx.send(ServerMessage::UnsignedDelegateTx {
                request_id: request_id.clone(),
                tx_base64: sig_preview.clone(),
                description: format!("Delegation submitted for {}", position_pda),
                submit_to: "pending".into(),
                position_pda: position_pda.clone(),
                owner_pubkey: owner_pubkey.clone(),
                market_id: market_id.clone(),
                nonce,
            });

            tokio::spawn(async move {
                match session.submit(close_tx).await {
                    Ok(sig) => {
                        let _ = broadcast.send(ServerMessage::CloseErSubmitted {
                            request_id: format!("{}_close", request_id),
                            signature: sig.to_string(),
                            position_pda: position_pda.clone(),
                        });

                        let er_bh = match session.recent_blockhash().await {
                            Ok(h) => h,
                            Err(e) => {
                                let _ = broadcast.send(ServerMessage::Error {
                                    code: "BLOCKHASH_ERROR".into(),
                                    message: e.to_string(),
                                });
                                return;
                            }
                        };

                        match builder.undelegate_position_tx(
                            &owner,
                            &position_pda_key,
                            mid,
                            nonce,
                            er_bh,
                        ) {
                            Ok(tx) => {
                                use base64::{engine::general_purpose, Engine as _};
                                let tx_base64 = general_purpose::STANDARD
                                    .encode(bincode::serialize(&tx).unwrap_or_default());
                                let _ = broadcast.send(ServerMessage::UnsignedUndelegateTx {
                                    request_id: format!("{}_undelegate", request_id),
                                    tx_base64,
                                    description: "Return position to Solana base layer".into(),
                                    submit_to: "er".into(),
                                });
                                session.remove_delegation(&position_pda_key);
                                if session.delegated.is_empty() {
                                    registry.remove(&owner);
                                }
                            }
                            Err(e) => {
                                let _ = broadcast.send(ServerMessage::Error {
                                    code: "TX_BUILD_ERROR".into(),
                                    message: e.to_string(),
                                });
                            }
                        }
                    }
                    Err(e) => {
                        let _ = broadcast.send(ServerMessage::Error {
                            code: "CLOSE_ER_FAILED".into(),
                            message: e.to_string(),
                        });
                    }
                }
            });
        }

        _ => {}
    }
}

fn market_id_to_bytes(s: &str) -> [u8; 16] {
    let mut arr = [0u8; 16];
    let bytes = s.as_bytes();
    let len = bytes.len().min(16);
    arr[..len].copy_from_slice(&bytes[..len]);
    arr
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
