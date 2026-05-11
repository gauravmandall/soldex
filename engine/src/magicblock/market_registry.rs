//! Fetches MarketState accounts from chain at startup and registers them with KeeperBot.

use std::sync::Arc;

use anchor_lang::AccountDeserialize;
use anyhow::{Context, Result};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use tracing::{error, info, warn};

use super::keeper::KeeperBot;
use super::price_updater::market_str_to_id;
use soldex_perps::state::MarketState;

/// Markets to register at startup.
/// Add new markets here when they're initialized on-chain.
const KNOWN_MARKETS: &[&str] = &["SOL-PERP", "BTC-PERP", "ETH-PERP", "JUP-PERP"];

/// Fetch known MarketState accounts and register with KeeperBot. Called once at startup.
pub async fn register_markets(
    keeper_bot: Arc<KeeperBot>,
    rpc: Arc<RpcClient>,
    program_id: &Pubkey,
    perps: Arc<crate::perps::PerpsEngine>,
) -> Result<()> {
    info!("Fetching market accounts from chain...");
    let mut registered = 0u32;

    for market_name in KNOWN_MARKETS {
        let market_id = market_str_to_id(market_name);

        // Derive MarketState PDA — seeds: [b"market", market_id]
        let (market_pda, _) =
            Pubkey::find_program_address(&[b"market", market_id.as_ref()], program_id);

        // Fetch account data from chain
        let account = match rpc.get_account(&market_pda).await {
            Ok(a) => a,
            Err(_) => {
                warn!(
                    market = market_name,
                    "MarketState not found on-chain — skipping"
                );
                continue;
            }
        };

        // Deserialize — anchor accounts have 8-byte discriminator prefix
        let mut data = account.data.as_slice();
        let market_state = match MarketState::try_deserialize(&mut data) {
            Ok(m) => m,
            Err(e) => {
                error!(
                    market = market_name,
                    "Failed to deserialize MarketState: {e}"
                );
                continue;
            }
        };

        if !market_state.is_active {
            warn!(market = market_name, "Market is inactive — skipping");
            continue;
        }

        keeper_bot.register_market(market_id, market_pda, market_state.price_feed);
        perps.market_cache.insert(
            market_id,
            crate::perps::CachedMarket {
                market_pda,
                price_feed: market_state.price_feed,
                quote_mint: market_state.quote_mint,
                vault: market_state.vault,
            },
        );
        info!(
            market = market_name,
            %market_pda,
            price_feed = %market_state.price_feed,
            "Market registered"
        );
        registered += 1;
    }

    info!(
        "Market registration complete: {registered}/{} markets",
        KNOWN_MARKETS.len()
    );
    Ok(())
}
