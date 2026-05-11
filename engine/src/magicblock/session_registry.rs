// magicblock/session_registry.rs

use std::sync::Arc;

use anyhow::Result;
use dashmap::DashMap;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use tracing::info;

use super::session::ErSession;
use super::tee_client::TeeClient;

/// Per-owner ER session registry — provisioned on ActivateSession, removed on undelegation.
pub struct SessionRegistry {
    sessions: DashMap<Pubkey, Arc<ErSession>>,
    keeper: Arc<Keypair>, // signs liquidation / close_er txs
    tee_auth_url: String,
    tee_rpc_url: String,
    er_rpc_url: String,
    er_ws_url: String,
    base_rpc_url: String,
}

impl SessionRegistry {
    pub fn new(
        keeper: Arc<Keypair>,
        tee_auth_url: String,
        tee_rpc_url: String,
        er_rpc_url: String,
        er_ws_url: String,
        base_rpc_url: String,
    ) -> Arc<Self> {
        Arc::new(Self {
            sessions: DashMap::new(),
            keeper,
            tee_auth_url,
            tee_rpc_url,
            er_rpc_url,
            er_ws_url,
            base_rpc_url,
        })
    }

    /// Return existing session or provision a new one with fresh TEE auth.
    pub async fn get_or_create(&self, owner: &Pubkey) -> Result<Arc<ErSession>> {
        if let Some(session) = self.sessions.get(owner) {
            return Ok(session.clone());
        }

        info!(%owner, "Provisioning new Private ER session");

        let tee_client = Arc::new(TeeClient::new(
            self.keeper.clone(),
            self.tee_auth_url.clone(),
            self.tee_rpc_url.clone(),
        ));
        tee_client.authenticate().await?;

        let session = ErSession::new(
            &self.er_rpc_url,
            &self.er_ws_url,
            &self.base_rpc_url,
            self.keeper.clone(),
            tee_client,
        )
        .await?;

        self.sessions.insert(*owner, session.clone());
        info!(%owner, "Private ER session ready");
        Ok(session)
    }

    /// Look up existing session without creating one.
    pub fn get(&self, owner: &Pubkey) -> Option<Arc<ErSession>> {
        self.sessions.get(owner).map(|s| s.clone())
    }

    /// Remove session after undelegation.
    pub fn remove(&self, owner: &Pubkey) {
        self.sessions.remove(owner);
        info!(%owner, "Private ER session removed");
    }

    /// Iterate all active sessions.
    pub fn iter_sessions(&self) -> Vec<(Pubkey, Arc<ErSession>)> {
        self.sessions
            .iter()
            .map(|e| (*e.key(), e.value().clone()))
            .collect()
    }

    pub fn active_count(&self) -> usize {
        self.sessions.len()
    }
}
