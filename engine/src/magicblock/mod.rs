/// MagicBlock ephemeral rollup — session manager, keeper bot, TEE client, price updater.
/// Re-exports all submodules for use across the engine.
/// 
pub mod session;
pub mod keeper;
pub mod tx_builder;
pub mod tee_client;   
pub mod price_updater;
pub mod market_registry;   
pub mod session_registry;


pub use session::ErSession;
pub use keeper::KeeperBot;
pub use tx_builder::ErTxBuilder;
pub use tee_client::TeeClient; 
pub use price_updater::run_price_updater; 
pub use market_registry::register_markets; 
pub use session_registry::SessionRegistry;       
