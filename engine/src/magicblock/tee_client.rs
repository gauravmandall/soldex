//! TEE auth client — challenge/sign/login flow, caches bearer token in memory.
//! Auto-retries on 401 by re-authenticating before each request.

use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use reqwest::{Client, Response, StatusCode};
use serde::{Deserialize, Serialize};
use solana_sdk::{signature::Keypair, signer::Signer};
use tokio::sync::RwLock;
use tracing::{info, warn};

// ── Wire types ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ChallengeResponse {
    challenge: String, // base58-encoded bytes
}

#[derive(Debug, Serialize)]
struct LoginRequest {
    pubkey: String,
    challenge: String,
    signature: String,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    token: String,
}

// ── TeeClient ────────────────────────────────────────────────────────────────

pub struct TeeClient {
    pub keeper: Arc<Keypair>,
    token: Arc<RwLock<String>>,
    pub tee_auth_url: String,
    pub tee_rpc_url: String,
    http: Client,
}

impl TeeClient {
    pub fn new(keeper: Arc<Keypair>, tee_auth_url: String, tee_rpc_url: String) -> Self {
        Self {
            keeper,
            token: Arc::new(RwLock::new(String::new())),
            tee_auth_url,
            tee_rpc_url,
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("reqwest client init failed"),
        }
    }

    // ── Auth ─────────────────────────────────────────────────────────────────

    pub async fn authenticate(&self) -> Result<()> {
        let pubkey = self.keeper.pubkey().to_string();

        let url = format!("{}/auth/challenge?pubkey={}", self.tee_auth_url, pubkey);
        let challenge: ChallengeResponse = self
            .http
            .get(&url)
            .send()
            .await
            .context("GET /v1/spl/challenge failed")?
            .error_for_status()
            .context("challenge endpoint error")?
            .json()
            .await
            .context("parse challenge failed")?;

        let bytes = challenge.challenge.as_bytes();

        let sig = self.keeper.sign_message(bytes);

        let login_url = format!("{}/auth/login", self.tee_auth_url);
        let resp: LoginResponse = self
            .http
            .post(&login_url)
            .json(&LoginRequest {
                pubkey,
                challenge: challenge.challenge,
                signature: sig.to_string(),
            })
            .send()
            .await
            .context("POST /v1/spl/login failed")?
            .error_for_status()
            .context("login endpoint error")?
            .json()
            .await
            .context("parse login response failed")?;

        *self.token.write().await = resp.token;
        info!(pubkey = %self.keeper.pubkey(), "TEE authenticated");
        Ok(())
    }

    async fn refresh(&self) -> Result<()> {
        warn!("TEE token expired — re-authenticating");
        self.authenticate().await
    }

    pub async fn current_token(&self) -> String {
        self.token.read().await.clone()
    }

    // ── Authenticated requests (auto-retry on 401) ────────────────────────

    pub async fn get(&self, url: &str) -> Result<Response> {
        let resp = self.raw_get(url).await?;
        if resp.status() == StatusCode::UNAUTHORIZED {
            self.refresh().await?;
            return self.raw_get(url).await;
        }
        Ok(resp)
    }

    pub async fn post_json<B: Serialize>(&self, url: &str, body: &B) -> Result<Response> {
        let resp = self.raw_post(url, body).await?;
        if resp.status() == StatusCode::UNAUTHORIZED {
            self.refresh().await?;
            return self.raw_post(url, body).await;
        }
        Ok(resp)
    }

    // ── Raw (no retry) ────────────────────────────────────────────────────

    async fn raw_get(&self, url: &str) -> Result<Response> {
        let token = self.current_token().await;
        self.http
            .get(url)
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await
            .context(format!("GET {url}"))
    }

    async fn raw_post<B: Serialize>(&self, url: &str, body: &B) -> Result<Response> {
        let token = self.current_token().await;
        self.http
            .post(url)
            .header("Authorization", format!("Bearer {token}"))
            .json(body)
            .send()
            .await
            .context(format!("POST {url}"))
    }
}
