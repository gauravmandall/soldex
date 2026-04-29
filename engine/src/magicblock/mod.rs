/// magicblock/mod.rs
///
/// MagicBlock ephemeral rollup session manager.
/// Manages the ER WebSocket connection, tracks delegated positions,
/// submits keeper transactions at ultra-low latency, and handles
/// session reconnection.
pub mod session;
pub mod keeper;
pub mod tx_builder;

pub use session::ErSession;
pub use keeper::KeeperBot;
pub use tx_builder::ErTxBuilder;