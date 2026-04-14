use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct EngineConfig {
    /// WebSocket bind address
    pub listen_addr: String,
    /// Helius / QuickNode RPC
    pub solana_rpc_url: String,
    /// Solana WebSocket RPC
    pub solana_ws_url: String,
    /// Deployed program ID
    pub program_id: String,
    /// Polymarket CLOB API
    pub polymarket_api_url: String,
    /// Polymarket WebSocket
    pub polymarket_ws_url: String,
    /// Encryption key for stored keypairs (hex-encoded 32 bytes)
    pub wallet_encryption_key: String,
    /// Max position size in USD
    pub max_position_usd: f64,
    /// Max leverage
    pub max_leverage: f64,
}

impl EngineConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            listen_addr: std::env::var("ENGINE_LISTEN_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:9000".into()),
            solana_rpc_url: std::env::var("SOLANA_RPC_URL")
                .unwrap_or_else(|_| "https://devnet.helius-rpc.com".into()),
            solana_ws_url: std::env::var("SOLANA_WS_URL")
                .unwrap_or_else(|_| "wss://devnet.helius-rpc.com".into()),
            program_id: std::env::var("PROGRAM_ID")
                .unwrap_or_else(|_| "REPLACE_WITH_PROGRAM_ID".into()),
            polymarket_api_url: std::env::var("POLYMARKET_API_URL")
                .unwrap_or_else(|_| "https://clob.polymarket.com".into()),
            polymarket_ws_url: std::env::var("POLYMARKET_WS_URL")
                .unwrap_or_else(|_| "wss://ws-subscriptions-clob.polymarket.com/ws/market".into()),
            wallet_encryption_key: std::env::var("WALLET_ENCRYPTION_KEY")
                .unwrap_or_else(|_| "0000000000000000000000000000000000000000000000000000000000000000".into()),
            max_position_usd: std::env::var("MAX_POSITION_USD")
                .unwrap_or_else(|_| "10000".into())
                .parse()?,
            max_leverage: std::env::var("MAX_LEVERAGE")
                .unwrap_or_else(|_| "20".into())
                .parse()?,
        })
    }
}
