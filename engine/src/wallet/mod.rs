/// Self-Custody Wallet Module
/// Manages encrypted keypairs server-side for ultra-fast tx signing.
/// Alternatively (preferred for privacy), the frontend holds keys and
/// sends signed txs to the engine for submission.

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use anyhow::{Context, Result};
use solana_sdk::signature::{Keypair, Signer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Encrypted keypair stored in memory (or persisted to DB)
#[derive(Debug ,Clone, Serialize, Deserialize)]
pub struct EncryptedKeypair {
    pub pubkey: String,          // base58
    pub encrypted_secret: Vec<u8>,
    pub nonce: Vec<u8>,
    pub created_at: u64,
    pub rotation_due_at: u64,   // unix ms — privacy rotation schedule
    pub label: String,
}

/// Rotation schedule: wallets auto-rotate after N days for privacy
const ROTATION_DAYS: u64 = 30;
const MS_PER_DAY: u64 = 86_400_000;

pub struct WalletManager {
    cipher: Aes256Gcm,
    /// pubkey → encrypted keypair (in-memory; real impl should use encrypted DB)
    wallets: RwLock<HashMap<String, EncryptedKeypair>>,
}

impl WalletManager {
    pub fn new(encryption_key_hex: &str) -> Result<Self> {
        let key_bytes = hex::decode(encryption_key_hex).context("invalid encryption key hex")?;
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        Ok(Self {
            cipher,
            wallets: RwLock::new(HashMap::new()),
        })
    }

    /// Generate a new keypair, encrypt it, and store it
    pub async fn generate_wallet(&self, label: &str) -> Result<EncryptedKeypair> {
        let keypair = Keypair::new();
        let pubkey = keypair.pubkey().to_string();

        // Encrypt the private key
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let secret_bytes = keypair.to_bytes().to_vec();
        let encrypted_secret = self
            .cipher
            .encrypt(&nonce, secret_bytes.as_ref())
            .map_err(|e| anyhow::anyhow!("encryption failed: {e}"))?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let ekp = EncryptedKeypair {
            pubkey: pubkey.clone(),
            encrypted_secret,
            nonce: nonce.to_vec(),
            created_at: now,
            rotation_due_at: now + ROTATION_DAYS * MS_PER_DAY,
            label: label.to_string(),
        };

        self.wallets.write().await.insert(pubkey, ekp.clone());
        Ok(ekp)
    }

    /// Sign a Solana transaction with a stored keypair
    pub async fn sign_transaction(&self, pubkey: &str, tx_bytes: &[u8]) -> Result<Vec<u8>> {
        let wallets = self.wallets.read().await;
        let ekp = wallets.get(pubkey).context("wallet not found")?;

        let nonce = Nonce::from_slice(&ekp.nonce);
        let secret_bytes = self
            .cipher
            .decrypt(nonce, ekp.encrypted_secret.as_ref())
            .map_err(|e| anyhow::anyhow!("decryption failed: {e}"))?;

        let keypair = Keypair::from_bytes(&secret_bytes)
            .map_err(|_| anyhow::anyhow!("invalid keypair bytes"))?;

        let signature = keypair.sign_message(tx_bytes);
        Ok(signature.as_ref().to_vec())
    }

    /// List all wallets (pubkeys + metadata, no secrets)
    pub async fn list_wallets(&self) -> Vec<EncryptedKeypair> {
        self.wallets.read().await.values().cloned().collect()
    }

    /// Check which wallets are due for rotation
    pub async fn rotation_candidates(&self) -> Vec<String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        self.wallets
            .read()
            .await
            .values()
            .filter(|w| w.rotation_due_at <= now)
            .map(|w| w.pubkey.clone())
            .collect()
    }
    /// Import an externally-generated Keypair, encrypt it, and store it.
    /// Use this once to migrate a plain keeper.json into the wallet store.
    pub async fn import_keypair(&self, label: &str, keypair: Keypair) -> Result<String> {
        let pubkey = keypair.pubkey().to_string();
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let encrypted_secret = self
            .cipher
            .encrypt(&nonce, keypair.to_bytes().as_ref())
            .map_err(|e| anyhow::anyhow!("encrypt failed: {e}"))?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        self.wallets.write().await.insert(
            pubkey.clone(),
            EncryptedKeypair {
                pubkey: pubkey.clone(),
                encrypted_secret,
                nonce: nonce.to_vec(),
                created_at: now,
                rotation_due_at: now + ROTATION_DAYS * MS_PER_DAY,
                label: label.to_string(),
            },
        );
        Ok(pubkey)
    }

    /// Decrypt and return an Arc<Keypair> ready for signing.
    pub async fn get_keypair(&self, pubkey: &str) -> Result<std::sync::Arc<Keypair>> {
        let wallets = self.wallets.read().await;
        let ekp = wallets.get(pubkey).context("keypair not found")?;
        let nonce = Nonce::from_slice(&ekp.nonce);
        let secret = self
            .cipher
            .decrypt(nonce, ekp.encrypted_secret.as_ref())
            .map_err(|e| anyhow::anyhow!("decrypt failed: {e}"))?;
        Ok(std::sync::Arc::new(
            Keypair::from_bytes(&secret).map_err(|e| anyhow::anyhow!("bad keypair bytes: {e}"))?,
        ))
    }

    /// Serialize the encrypted keypair to a JSON file.
    /// File contains only ciphertext — safe to store, never contains raw secret.
    pub async fn save_keypair_to_file(
        &self,
        pubkey: &str,
        path: &str,
    ) -> Result<()> {
        let wallets = self.wallets.read().await;
        let ekp = wallets.get(pubkey).context("keypair not found")?;
        let json = serde_json::to_string_pretty(ekp)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load an encrypted keypair JSON file into the in-memory store.
    /// Verifies decryption succeeds before inserting — catches wrong key early.
    pub async fn load_keypair_from_file(&self, path: &str) -> Result<String> {
        let json = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {path}"))?;
        let ekp: EncryptedKeypair = serde_json::from_str(&json)?;

        // Verify decrypt works before we trust this file
        let nonce = Nonce::from_slice(&ekp.nonce);
        self.cipher
            .decrypt(nonce, ekp.encrypted_secret.as_ref())
            .map_err(|_| anyhow::anyhow!("decrypt failed — wrong key or corrupted file: {path}"))?;

        let pubkey = ekp.pubkey.clone();
        self.wallets.write().await.insert(pubkey.clone(), ekp);
        Ok(pubkey)
    }
}
