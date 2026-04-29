/// ws/handler.rs
///
/// Routes incoming WebSocket messages to the appropriate subsystem.
/// MagicBlock delegation flow:
///   1. Client sends DelegatePosition  → engine builds unsigned tx → client signs → submits to base layer
///   2. Engine detects delegation confirm → registers in ErSession
///   3. Keeper sends ER updates → engine streams ErPositionUpdate to client
///   4. Client sends CloseAndUndelegatePosition → engine builds close tx (ER) + undelegate tx
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use base64::Engine as _;
use solana_sdk::{pubkey::Pubkey, transaction::Transaction};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use super::messages::*;
use crate::magicblock::session::{DelegatedPosition, ErSession};
use crate::magicblock::tx_builder::ErTxBuilder;
// TODO: needs price_feed module in perps/

pub struct WsHandler {
    er_session: Arc<ErSession>,
    tx_builder: Arc<ErTxBuilder>,
    price_feed: Arc<PriceFeed>,
    /// Sender to push messages back to the WS client connection
    client_tx: mpsc::Sender<ServerMsg>,
}

impl WsHandler {
    pub fn new(
        er_session: Arc<ErSession>,
        tx_builder: Arc<ErTxBuilder>,
        price_feed: Arc<PriceFeed>,
        client_tx: mpsc::Sender<ServerMsg>,
    ) -> Self {
        Self {
            er_session,
            tx_builder,
            price_feed,
            client_tx,
        }
    }

    pub async fn handle(&self, msg: ClientMsg) {
        let result = match msg {
            ClientMsg::Ping => {
                let _ = self.client_tx.send(ServerMsg::Pong).await;
                Ok(())
            }
            ClientMsg::DelegatePosition(m) => self.handle_delegate(m).await,
            ClientMsg::CloseAndUndelegatePosition(m) => self.handle_close_undelegate(m).await,
            ClientMsg::GetDelegatedPosition { position_pda } => {
                self.handle_get_delegated(position_pda).await
            }
            // Other handlers (orders, polymarket, etc.) omitted for brevity
            _ => Ok(()),
        };

        if let Err(e) = result {
            error!("ws handler error: {}", e);
            let _ = self
                .client_tx
                .send(ServerMsg::Error {
                    code: "HANDLER_ERROR".into(),
                    message: e.to_string(),
                })
                .await;
        }
    }

    // ── DelegatePosition ─────────────────────────────────────────────────────

    async fn handle_delegate(&self, msg: DelegatePositionMsg) -> Result<()> {
        let owner: Pubkey = msg.owner_pubkey.parse()?;
        let position_pda: Pubkey = msg.position_pda.parse()?;
        let market_id = market_id_from_str(&msg.market_id);
        let validator: Option<Pubkey> = msg
            .validator_pubkey
            .as_deref()
            .map(|s| s.parse())
            .transpose()?;

        let blockhash = self.er_session.recent_blockhash().await?;

        let tx = self.tx_builder.delegate_position_tx(
            &owner,
            &position_pda,
            market_id,
            validator,
            blockhash,
        )?;

        let tx_bytes = bincode::serialize(&tx)?;
        let tx_base64 = base64::engine::general_purpose::STANDARD.encode(&tx_bytes);

        self.client_tx
            .send(ServerMsg::UnsignedDelegateTx(UnsignedTxMsg {
                request_id: msg.request_id,
                tx_base64,
                description: format!(
                    "Delegate {} position to MagicBlock ER for ultra-low latency trading",
                    msg.market_id
                ),
                submit_to: SubmitTarget::Base, // delegation tx goes to base layer
            }))
            .await?;

        // Watch for confirmation and register in ER session
        // In production, subscribe to base-layer tx confirmation webhook
        // Here we optimistically register — confirmation listener updates if it fails
        let er_rpc = self.er_session_rpc_url();
        let client_tx = self.client_tx.clone();
        let session = self.er_session.clone();

        tokio::spawn(async move {
            // TODO: replace with actual confirmation listener (Helius webhook / geyser)
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

            let delegated = DelegatedPosition {
                position_pda,
                owner,
                market_id,
                size: 0,       // will be updated from chain
                entry_price: 0,
                is_long: true,
                collateral: 0,
                delegated_at: unix_now(),
                last_mark_price: 0,
                last_update_at: std::time::Instant::now(),
            };
            session.register_delegation(delegated);

            let _ = client_tx
                .send(ServerMsg::DelegationConfirmed(DelegationConfirmedMsg {
                    owner: owner.to_string(),
                    position_pda: position_pda.to_string(),
                    market_id: msg.market_id,
                    er_rpc_url: er_rpc,
                    delegated_at: unix_now(),
                }))
                .await;
        });

        Ok(())
    }

    // ── CloseAndUndelegatePosition ────────────────────────────────────────────

    async fn handle_close_undelegate(&self, msg: CloseUndelegateMsg) -> Result<()> {
        let owner: Pubkey = msg.owner_pubkey.parse()?;
        let position_pda: Pubkey = msg.position_pda.parse()?;
        let market_id = market_id_from_str(&msg.market_id);

        // Validate exit price against oracle (allow 1% slippage)
        let oracle_price = self
            .price_feed
            .latest_price(&market_id)
            .await
            .ok_or_else(|| anyhow::anyhow!("no oracle price available"))?;

        let slippage_bps = if msg.exit_price > oracle_price {
            ((msg.exit_price - oracle_price) as u128 * 10_000 / oracle_price as u128) as u64
        } else {
            ((oracle_price - msg.exit_price) as u128 * 10_000 / oracle_price as u128) as u64
        };

        if slippage_bps > 100 {
            // > 1% slippage
            anyhow::bail!(
                "exit price {} deviates {:.2}% from oracle {}",
                msg.exit_price,
                slippage_bps as f64 / 100.0,
                oracle_price
            );
        }

        // Step 1: build close_position_er tx (submit to ER, user signs)
        let er_blockhash = self.er_session.recent_blockhash().await?;
        let close_tx = self.tx_builder.close_position_er_tx(
            &owner,
            &position_pda,
            market_id,
            msg.exit_price,
            0i128, // funding index: fetched from chain in production
            er_blockhash,
        )?;

        let close_b64 = base64::engine::general_purpose::STANDARD
            .encode(bincode::serialize(&close_tx)?);

        self.client_tx
            .send(ServerMsg::UnsignedCloseErTx(UnsignedTxMsg {
                request_id: format!("{}_close", msg.request_id),
                tx_base64: close_b64,
                description: format!(
                    "Close {} position at ${:.4} (ER fast path)",
                    msg.market_id,
                    msg.exit_price as f64 / 1_000_000.0
                ),
                submit_to: SubmitTarget::Er,
            }))
            .await?;

        // Step 2: build undelegate_position tx (submit to ER after close confirms)
        let undelegate_tx = self.tx_builder.undelegate_position_tx(
            &owner,
            &position_pda,
            market_id,
            er_blockhash,
        )?;

        let undelegate_b64 = base64::engine::general_purpose::STANDARD
            .encode(bincode::serialize(&undelegate_tx)?);

        self.client_tx
            .send(ServerMsg::UnsignedUndelegateTx(UnsignedTxMsg {
                request_id: format!("{}_undelegate", msg.request_id),
                tx_base64: undelegate_b64,
                description: "Return position to Solana base layer".into(),
                submit_to: SubmitTarget::Er,
            }))
            .await?;

        // Optimistically remove from delegated map (real removal on confirmation)
        self.er_session.remove_delegation(&position_pda);

        Ok(())
    }

    // ── GetDelegatedPosition ──────────────────────────────────────────────────

    async fn handle_get_delegated(&self, position_pda_str: String) -> Result<()> {
        let pda: Pubkey = position_pda_str.parse()?;

        if let Some(pos) = self.er_session.delegated.get(&pda) {
            let oracle_price = self
                .price_feed
                .latest_price(&pos.market_id)
                .await
                .unwrap_or(pos.last_mark_price);

            let notional = (pos.size as u128)
                .saturating_mul(oracle_price as u128)
                / 1_000_000;

            let margin_ratio_bps = if notional > 0 {
                (pos.collateral as u128).saturating_mul(10_000) / notional
            } else {
                u128::MAX
            } as u64;

            let pnl: i64 = if pos.is_long {
                (oracle_price as i64 - pos.entry_price as i64)
                    .saturating_mul(pos.size as i64)
                    .saturating_div(1_000_000)
            } else {
                (pos.entry_price as i64 - oracle_price as i64)
                    .saturating_mul(pos.size as i64)
                    .saturating_div(1_000_000)
            };

            self.client_tx
                .send(ServerMsg::ErPositionUpdate(ErPositionUpdateMsg {
                    position_pda: pda.to_string(),
                    market_id: String::from_utf8_lossy(&pos.market_id)
                        .trim_matches('\0')
                        .to_string(),
                    size: pos.size,
                    mark_price: oracle_price,
                    unrealised_pnl: pnl,
                    margin_ratio_bps,
                    pending_funding: 0,
                    ts_ms: unix_ms(),
                }))
                .await?;
        } else {
            anyhow::bail!("position {} not found in ER", position_pda_str);
        }

        Ok(())
    }

    fn er_session_rpc_url(&self) -> String {
        // In production, return the actual ER validator RPC URL
        "https://devnet.magicblock.app".into()
    }
}

/// Stream ER position updates to a subscribed client. Called by the keeper
/// after each update_position_er tx confirms.
pub async fn stream_er_update(
    client_tx: &mpsc::Sender<ServerMsg>,
    pda: &Pubkey,
    market_id: &[u8; 16],
    size: u64,
    mark_price: u64,
    unrealised_pnl: i64,
    margin_ratio_bps: u64,
    pending_funding: i64,
) {
    let msg = ServerMsg::ErPositionUpdate(ErPositionUpdateMsg {
        position_pda: pda.to_string(),
        market_id: String::from_utf8_lossy(market_id)
            .trim_matches('\0')
            .to_string(),
        size,
        mark_price,
        unrealised_pnl,
        margin_ratio_bps,
        pending_funding,
        ts_ms: unix_ms(),
    });
    let _ = client_tx.send(msg).await;
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}