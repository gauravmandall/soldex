use std::sync::Arc;
use tokio::sync::RwLock;
use solana_sdk::hash::Hash;

#[derive(Clone)]
pub struct BlockhashCache {
    pub latest: Arc<RwLock<Hash>>,
}

impl BlockhashCache {
    pub fn new(initial: Hash) -> Self {
        Self {
            latest: Arc::new(RwLock::new(initial)),
        }
    }

    pub async fn get(&self) -> Hash {
        *self.latest.read().await
    }

    pub async fn set(&self, hash: Hash) {
        *self.latest.write().await = hash;
    }
}