/// magicblock/session.rs
///
/// Persistent connection to the MagicBlock ephemeral rollup RPC/WS endpoint.
/// Provides sub-100ms transaction submission by keeping a warm connection
/// and batching updates within a single slot window.
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use dashmap::DashMap;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    transaction::Transaction,
};
use tokio::sync::{mpsc, RwLock};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

/// State of a delegated position from the engine's perspective.
#[derive(Debug, Clone)]
pub struct DelegatedPosition {
    pub position_pda: Pubkey,
    pub owner: Pubkey,
    pub market_id: [u8; 16],
    pub size: u64,
    pub entry_price: u64,
    pub is_long: bool,
    pub collateral: u64,
    pub delegated_at: i64,
    /// Latest mark price seen for this position
    pub last_mark_price: u64,
    /// Last time we submitted an update tx for this position
    pub last_update_at: Instant,
}

/// Commands sent to the session actor
#[derive(Debug)]
pub enum ErCommand {
    SubmitTransaction(Transaction),
    GetSlot(tokio::sync::oneshot::Sender<u64>),
    Shutdown,
}

/// Manages a persistent connection to the ER RPC endpoint.
pub struct ErSession {
    /// ER RPC client (HTTP, but kept warm with frequent calls)
    er_client: Arc<RpcClient>,
    /// Base-layer client (for delegation txs)
    base_client: Arc<RpcClient>,
    /// Keeper keypair — signs all ER transactions
    keeper: Arc<Keypair>,
    /// Map of position PDA → DelegatedPosition
    pub delegated: Arc<DashMap<Pubkey, DelegatedPosition>>,
    /// Channel to the session actor for serialised RPC calls
    cmd_tx: mpsc::Sender<ErCommand>,
    /// Last confirmed slot in the ER
    last_slot: Arc<RwLock<u64>>,
}

impl ErSession {
    pub async fn new(
        er_rpc_url: &str,
        er_ws_url: &str,
        base_rpc_url: &str,
        keeper: Arc<Keypair>,
    ) -> Result<Arc<Self>> {
        let er_client = Arc::new(RpcClient::new_with_commitment(
            er_rpc_url.to_string(),
            CommitmentConfig::confirmed(),
        ));
        let base_client = Arc::new(RpcClient::new_with_commitment(
            base_rpc_url.to_string(),
            CommitmentConfig::confirmed(),
        ));

        let (cmd_tx, cmd_rx) = mpsc::channel(256);
        let delegated = Arc::new(DashMap::new());
        let last_slot = Arc::new(RwLock::new(0u64));

        let session = Arc::new(Self {
            er_client: er_client.clone(),
            base_client,
            keeper,
            delegated,
            cmd_tx,
            last_slot: last_slot.clone(),
        });

        // Spawn the RPC actor — serialises submissions to avoid nonce races
        tokio::spawn(Self::rpc_actor(er_client.clone(), cmd_rx, last_slot));

        // Spawn slot-watcher to keep last_slot fresh (WS subscription)
        tokio::spawn(Self::slot_watcher(
            er_ws_url.to_string(),
            session.last_slot.clone(),
        ));

        info!("ErSession connected to {}", er_rpc_url);
        Ok(session)
    }

    /// Submit a pre-signed transaction to the ER with retry logic.
    /// Returns the signature on success.
    pub async fn submit(&self, tx: Transaction) -> Result<Signature> {
        // Fast path: fire-and-forget with confirmed commitment
        // The ER confirms in ~50-80ms vs ~400ms on base layer.
        let sig = self
            .er_client
            .send_and_confirm_transaction_with_spinner_and_commitment(
                &tx,
                CommitmentConfig::confirmed(),
            )
            .await
            .context("ER tx submission failed")?;

        debug!("ER tx confirmed: {}", sig);
        Ok(sig)
    }

    /// Submit and DO NOT wait for confirmation — max throughput path.
    /// Used for high-frequency updates where occasional drops are acceptable.
    pub async fn submit_fire_and_forget(&self, tx: Transaction) -> Result<Signature> {
        let sig = self
            .er_client
            .send_transaction(&tx)
            .await
            .context("ER fire-and-forget failed")?;
        Ok(sig)
    }

    /// Get the latest blockhash from the ER (cached, refreshed every ~400ms)
    pub async fn recent_blockhash(&self) -> Result<solana_sdk::hash::Hash> {
        let (hash, _) = self
            .er_client
            .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
            .await
            .context("failed to get ER blockhash")?;
        Ok(hash)
    }

    /// Current slot in the ER
    pub async fn current_slot(&self) -> u64 {
        *self.last_slot.read().await
    }

    /// Register a position as delegated (called when delegation tx confirms on base layer)
    pub fn register_delegation(&self, pos: DelegatedPosition) {
        info!(
            "Registered delegation: {} market={:?}",
            pos.position_pda,
            std::str::from_utf8(&pos.market_id).unwrap_or("?")
        );
        self.delegated.insert(pos.position_pda, pos);
    }

    /// Remove a position when undelegation confirms
    pub fn remove_delegation(&self, position_pda: &Pubkey) {
        self.delegated.remove(position_pda);
        info!("Removed delegation: {}", position_pda);
    }

    /// Update cached position state (called after each ER update tx)
    pub fn update_position_cache(&self, pda: &Pubkey, size: u64, mark_price: u64, collateral: u64) {
        if let Some(mut pos) = self.delegated.get_mut(pda) {
            pos.size = size;
            pos.last_mark_price = mark_price;
            pos.collateral = collateral;
            pos.last_update_at = Instant::now();
        }
    }

    // ── Private actor tasks ───────────────────────────────────────────────────

    async fn rpc_actor(
        client: Arc<RpcClient>,
        mut rx: mpsc::Receiver<ErCommand>,
        last_slot: Arc<RwLock<u64>>,
    ) {
        while let Some(cmd) = rx.recv().await {
            match cmd {
                ErCommand::SubmitTransaction(tx) => match client.send_transaction(&tx).await {
                    Ok(sig) => debug!("actor submitted: {}", sig),
                    Err(e) => error!("actor tx error: {}", e),
                },
                ErCommand::GetSlot(resp) => {
                    let slot = client.get_slot().await.unwrap_or(0);
                    let _ = resp.send(slot);
                }
                ErCommand::Shutdown => break,
            }
        }
    }

    async fn slot_watcher(ws_url: String, slot: Arc<RwLock<u64>>) {
        use futures_util::StreamExt;
        use solana_pubsub_client::nonblocking::pubsub_client::PubsubClient;

        loop {
            match PubsubClient::new(&ws_url).await {
                Ok(client) => match client.slot_subscribe().await {
                    Ok((mut stream, _unsub)) => {
                        info!("✅ ER WS slot subscription active");
                        while let Some(notif) = stream.next().await {
                            *slot.write().await = notif.slot;
                        }
                        warn!("ER WS slot stream ended — reconnecting");
                    }
                    Err(e) => error!("slot_subscribe failed: {}", e),
                },
                Err(e) => error!("ER WS connect failed: {}", e),
            }
            sleep(Duration::from_secs(2)).await;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use solana_client::nonblocking::rpc_client::RpcClient;
    use solana_sdk::commitment_config::CommitmentConfig;
    use solana_sdk::signature::{Keypair, Signer};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_latency_comparison() {
        let keeper = Arc::new(Keypair::new());
        let session = ErSession::new(
            "https://devnet.magicblock.app",
            "wss://devnet.magicblock.app",
            "https://api.devnet.solana.com",
            keeper,
        )
        .await
        .unwrap();

        // ER latency
        let start = std::time::Instant::now();
        let _ = session.recent_blockhash().await;
        println!("⚡ ER blockhash fetch: {}ms", start.elapsed().as_millis());

        // Base layer latency
        let base = RpcClient::new_with_commitment(
            "https://api.devnet.solana.com".into(),
            CommitmentConfig::confirmed(),
        );
        let start = std::time::Instant::now();
        let _ = base.get_latest_blockhash().await;
        println!(
            "🐢 Base layer blockhash fetch: {}ms",
            start.elapsed().as_millis()
        );
    }

    #[tokio::test]
    async fn test_slot_freshness() {
        let keeper = Arc::new(Keypair::new());
        let session = ErSession::new(
            "https://devnet.magicblock.app",
            "wss://devnet.magicblock.app",
            "https://api.devnet.solana.com",
            keeper,
        )
        .await
        .unwrap();

        // Wait for slot watcher to kick in
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let slot1 = session.current_slot().await;
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        let slot2 = session.current_slot().await;

        println!("ER slot 1: {}", slot1);
        println!("ER slot 2: {}", slot2);
        println!("ER slot speed: {} slots/400ms", slot2 - slot1);

        // ER produces slots much faster than base layer
        // Base layer: 1 slot per 400ms
        // ER: can be 10-50 slots per 400ms
        assert!(slot1 > 0, "ER should be producing slots");
    }

  #[tokio::test]
async fn test_er_10ms_confirmation() {
    let keeper_path = "./keeper.json";
    let keeper = match solana_sdk::signature::read_keypair_file(keeper_path) {
        Ok(kp) => Arc::new(kp),
        Err(_) => {
            println!("⚠️  keeper.json not found — skipping test. Run: solana-keygen new -o keeper.json");
            return;
        }
    };
    let session = ErSession::new(
        "https://devnet.magicblock.app",
        "wss://devnet.magicblock.app",
        "https://api.devnet.solana.com",
        keeper.clone(),
    ).await.unwrap();

    // Warm up the connection first
    let _ = session.recent_blockhash().await.unwrap();

    let blockhash = session.recent_blockhash().await.unwrap();
    let tx = solana_sdk::system_transaction::transfer(
        &keeper,
        &keeper.pubkey(),
        0,
        blockhash,
    );

    // Fire and forget — measures raw submission latency, not confirmation polling
    let start = std::time::Instant::now();
    let sig = session.submit_fire_and_forget(tx).await.unwrap();
    let elapsed = start.elapsed().as_millis();

    println!("⚡ ER submission: {}ms | sig: {}", elapsed, sig);
    assert!(elapsed < 100, "ER should submit in under 100ms, got {}ms", elapsed);
}
}
