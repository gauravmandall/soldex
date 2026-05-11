/// ER session — TEE-authenticated RPC connection to MagicBlock ephemeral rollup.
/// Handles tx submission, token refresh on 401, slot watching, and delegated position cache.
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

use super::tee_client::TeeClient;

#[derive(Debug, Clone)]
pub struct DelegatedPosition {
    pub position_pda: Pubkey,
    pub owner: Pubkey,
    pub market_id: [u8; 16],
    pub nonce: u8,
    pub size: u64,
    pub entry_price: u64,
    pub is_long: bool,
    pub collateral: u64,
    pub delegated_at: i64,
    pub last_mark_price: u64,
    pub last_update_at: Instant,
}

#[derive(Debug)]
pub enum ErCommand {
    SubmitTransaction(Transaction),
    GetSlot(tokio::sync::oneshot::Sender<u64>),
    Shutdown,
}

pub struct ErSession {
    /// TEE-authenticated ER RPC client — wrapped in RwLock so we can
    /// swap it after a token refresh without restarting the session.
    er_client: Arc<RwLock<Arc<RpcClient>>>,
    /// Base-layer client (for delegation txs — no auth needed)
    base_client: Arc<RpcClient>,
    /// Keeper keypair — signs all ER transactions
    pub keeper: Arc<Keypair>,
    /// TEE client — holds bearer token, used to rebuild er_client on 401
    tee_client: Arc<TeeClient>,
    /// TEE RPC base URL (without token — token appended at build time)
    tee_rpc_url: String,
    pub delegated: Arc<DashMap<Pubkey, DelegatedPosition>>,
    cmd_tx: mpsc::Sender<ErCommand>,
    last_slot: Arc<RwLock<u64>>,
}

impl ErSession {
    pub async fn new(
        _er_rpc_url: &str, // kept for signature compat — TEE URL used instead
        er_ws_url: &str,
        base_rpc_url: &str,
        keeper: Arc<Keypair>,
        tee_client: Arc<TeeClient>,
    ) -> Result<Arc<Self>> {
        let tee_rpc_url = tee_client.tee_rpc_url.clone();

        let er_client = Arc::new(RwLock::new(Self::build_er_client(
            &tee_rpc_url,
            &tee_client.current_token().await,
        )));

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
            tee_client,
            tee_rpc_url,
            delegated,
            cmd_tx,
            last_slot: last_slot.clone(),
        });

        tokio::spawn(Self::rpc_actor(er_client, cmd_rx, last_slot));
        tokio::spawn(Self::slot_watcher(
            er_ws_url.to_string(),
            session.last_slot.clone(),
        ));

        info!("ErSession connected to TEE RPC");
        Ok(session)
    }

    /// Build an RpcClient with the token embedded as a query param.
    /// MagicBlock TEE RPC format: {base_url}?token={bearer_token}
    fn build_er_client(tee_rpc_url: &str, token: &str) -> Arc<RpcClient> {
        let url = format!("{}?token={}", tee_rpc_url, token);
        Arc::new(RpcClient::new_with_commitment(
            url,
            CommitmentConfig::confirmed(),
        ))
    }

    /// Refresh the bearer token and rebuild the ER client.
    /// Called automatically when a 401/auth error is detected.
    async fn refresh_er_client(&self) -> Result<()> {
        self.tee_client.authenticate().await?;
        let token = self.tee_client.current_token().await;
        let new_client = Self::build_er_client(&self.tee_rpc_url, &token);
        *self.er_client.write().await = new_client;
        info!("ER client rebuilt with fresh TEE token");
        Ok(())
    }

    /// Submit a pre-signed transaction to the ER.
    /// Retries once with a fresh token if the first attempt looks like an auth failure.
    pub async fn submit(&self, tx: Transaction) -> Result<Signature> {
        let client = self.er_client.read().await.clone();
        let result = client
            .send_and_confirm_transaction_with_spinner_and_commitment(
                &tx,
                CommitmentConfig::confirmed(),
            )
            .await;

        match result {
            Ok(sig) => {
                debug!("ER tx confirmed: {}", sig);
                Ok(sig)
            }
            Err(e) if Self::is_auth_error(&e) => {
                warn!("ER auth error — refreshing token and retrying");
                self.refresh_er_client().await?;
                let client = self.er_client.read().await.clone();
                client
                    .send_and_confirm_transaction_with_spinner_and_commitment(
                        &tx,
                        CommitmentConfig::confirmed(),
                    )
                    .await
                    .context("ER tx failed after token refresh")
            }
            Err(e) => Err(e).context("ER tx submission failed"),
        }
    }

    /// Fire-and-forget — max throughput, no confirmation wait.
    pub async fn submit_fire_and_forget(&self, tx: Transaction) -> Result<Signature> {
        let client = self.er_client.read().await.clone();
        let result = client.send_transaction(&tx).await;

        match result {
            Ok(sig) => Ok(sig),
            Err(e) if Self::is_auth_error(&e) => {
                self.refresh_er_client().await?;
                let client = self.er_client.read().await.clone();
                client
                    .send_transaction(&tx)
                    .await
                    .context("ER fire-and-forget failed after token refresh")
            }
            Err(e) => Err(e).context("ER fire-and-forget failed"),
        }
    }

    pub async fn recent_blockhash(&self) -> Result<solana_sdk::hash::Hash> {
        let client = self.er_client.read().await.clone();
        let (hash, _) = client
            .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
            .await
            .context("failed to get ER blockhash")?;
        Ok(hash)
    }

    pub async fn current_slot(&self) -> u64 {
        *self.last_slot.read().await
    }

    pub fn register_delegation(&self, pos: DelegatedPosition) {
        info!(
            "Registered delegation: {} market={:?}",
            pos.position_pda,
            std::str::from_utf8(&pos.market_id).unwrap_or("?")
        );
        self.delegated.insert(pos.position_pda, pos);
    }

    pub fn remove_delegation(&self, position_pda: &Pubkey) {
        self.delegated.remove(position_pda);
        info!("Removed delegation: {}", position_pda);
    }

    pub fn update_position_cache(&self, pda: &Pubkey, size: u64, mark_price: u64, collateral: u64) {
        if let Some(mut pos) = self.delegated.get_mut(pda) {
            pos.size = size;
            pos.last_mark_price = mark_price;
            pos.collateral = collateral;
            pos.last_update_at = Instant::now();
        }
    }

    /// Heuristic: treat connection refused / 401 / forbidden as auth errors.
    fn is_auth_error(e: &solana_client::client_error::ClientError) -> bool {
        let msg = e.to_string().to_lowercase();
        msg.contains("401")
            || msg.contains("403")
            || msg.contains("unauthorized")
            || msg.contains("forbidden")
    }

    // private actor tasks
    async fn rpc_actor(
        client: Arc<RwLock<Arc<RpcClient>>>,
        mut rx: mpsc::Receiver<ErCommand>,
        last_slot: Arc<RwLock<u64>>,
    ) {
        while let Some(cmd) = rx.recv().await {
            match cmd {
                ErCommand::SubmitTransaction(tx) => {
                    let c = client.read().await.clone();
                    match c.send_transaction(&tx).await {
                        Ok(sig) => debug!("actor submitted: {}", sig),
                        Err(e) => error!("actor tx error: {}", e),
                    }
                }
                ErCommand::GetSlot(resp) => {
                    let c = client.read().await.clone();
                    let slot = c.get_slot().await.unwrap_or(0);
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

    async fn make_test_session() -> Arc<ErSession> {
        let keeper = Arc::new(Keypair::new());
        // Tests hit the public ER directly — no TEE auth needed for latency checks
        let tee_client = Arc::new(TeeClient::new(
            keeper.clone(),
            "https://devnet-tee.magicblock.app".into(),
            "https://devnet.magicblock.app".into(), // public URL for tests
        ));
        ErSession::new(
            "https://devnet.magicblock.app",
            "wss://devnet.magicblock.app",
            "https://api.devnet.solana.com",
            keeper,
            tee_client,
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn test_latency_comparison() {
        let session = make_test_session().await;

        let start = std::time::Instant::now();
        let _ = session.recent_blockhash().await;
        println!("⚡ ER blockhash fetch: {}ms", start.elapsed().as_millis());

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
        let session = make_test_session().await;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let slot1 = session.current_slot().await;
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        let slot2 = session.current_slot().await;

        println!("ER slot 1: {}", slot1);
        println!("ER slot 2: {}", slot2);
        println!("ER slot speed: {} slots/400ms", slot2 - slot1);
        assert!(slot1 > 0, "ER should be producing slots");
    }

    #[tokio::test]
    async fn test_er_10ms_confirmation() {
        let keeper_path = "./keeper.json";
        let keeper = match solana_sdk::signature::read_keypair_file(keeper_path) {
            Ok(kp) => Arc::new(kp),
            Err(_) => {
                println!("⚠️  keeper.json not found — skipping test");
                return;
            }
        };
        let tee_client = Arc::new(TeeClient::new(
            keeper.clone(),
            "https://devnet-tee.magicblock.app".into(),
            "https://devnet.magicblock.app".into(),
        ));
        let session = ErSession::new(
            "https://devnet.magicblock.app",
            "wss://devnet.magicblock.app",
            "https://api.devnet.solana.com",
            keeper.clone(),
            tee_client,
        )
        .await
        .unwrap();

        let _ = session.recent_blockhash().await.unwrap();
        let blockhash = session.recent_blockhash().await.unwrap();
        let tx = solana_sdk::system_transaction::transfer(&keeper, &keeper.pubkey(), 0, blockhash);

        let start = std::time::Instant::now();
        let sig = session.submit_fire_and_forget(tx).await.unwrap();
        let elapsed = start.elapsed().as_millis();

        println!("⚡ ER submission: {}ms | sig: {}", elapsed, sig);
        assert!(
            elapsed < 100,
            "ER should submit in under 100ms, got {}ms",
            elapsed
        );
    }
}
