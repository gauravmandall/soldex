//! MagicBlock ER transaction builder.
//! Base layer txs (~400ms): delegate, undelegate
//! ER txs (~10ms): update_market_er, close_er, funding_er, liquidate_er

use std::sync::Arc;

use anchor_lang::{InstructionData, ToAccountMetas};
use anyhow::Result;
use solana_sdk::{
    hash::Hash, instruction::Instruction, message::Message, pubkey::Pubkey, signature::Keypair,
    signer::Signer, transaction::Transaction,
};

use crate::config::EngineConfig;

// Fixed MagicBlock program IDs — same on devnet and mainnet
fn magicblock_program_id() -> Pubkey {
    "Magic11111111111111111111111111111111111111"
        .parse()
        .expect("static")
}

fn delegation_program_id() -> Pubkey {
    "DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh"
        .parse()
        .expect("static")
}

// Global PDAs — computed once at startup, reused on every tx
pub struct CachedAccounts {
    pub magic_context: Pubkey, // seeds: ["magic_context"] under magicblock program
}

pub struct ErTxBuilder {
    program_id: Pubkey,
    keeper: Arc<Keypair>, // signs keeper txs (update_er, liquidate_er, funding_er)
    cache: CachedAccounts,
}

impl ErTxBuilder {
    pub fn new(config: &EngineConfig, keeper: Arc<Keypair>) -> Self {
        let (magic_context, _) =
            Pubkey::find_program_address(&[b"magic_context"], &magicblock_program_id());
        Self {
            program_id: config.program_id.parse().expect("invalid program_id"),
            keeper,
            cache: CachedAccounts { magic_context },
        }
    }

    // Derive position PDA — must match seeds in ctx_accounts.rs
    // seeds: [b"position", market_id, owner, nonce]
    pub fn derive_position_pda(&self, owner: &Pubkey, market_id: &[u8; 16], nonce: u8) -> Pubkey {
        Pubkey::find_program_address(
            &[b"position", market_id.as_ref(), owner.as_ref(), &[nonce]],
            &self.program_id,
        )
        .0
    }

    // ── BASE LAYER ────────────────────────────────────────────────────────────
    // Send these to Solana RPC. Happen once per position lifecycle.

    /// Unsigned tx — frontend signs. Send to BASE LAYER RPC.
    /// Moves position PDA to ER. After this, updates go through ER at 10ms.
    pub fn delegate_position_tx(
        &self,
        owner: &Pubkey,
        position_pda: &Pubkey,
        market_id: [u8; 16],
        nonce: u8,
        recent_blockhash: Hash,
    ) -> Result<Transaction> {
        // PDAs derived manually — must match seeds in ctx_accounts.rs
        let (buffer, _) =
            Pubkey::find_program_address(&[b"buffer", position_pda.as_ref()], &self.program_id);
        let (delegation_record, _) = Pubkey::find_program_address(
            &[b"delegation", position_pda.as_ref()],
            &delegation_program_id(),
        );
        let (delegation_metadata, _) = Pubkey::find_program_address(
            &[b"delegation-metadata", position_pda.as_ref()],
            &delegation_program_id(),
        );

        let accounts = soldex_perps::accounts::DelegatePosition {
            position: *position_pda,
            owner: *owner,
            buffer_position: buffer,
            delegation_record_position: delegation_record,
            delegation_metadata_position: delegation_metadata,
            owner_program: self.program_id,
            delegation_program: delegation_program_id(),
            system_program: solana_sdk::system_program::id(),
            permission_program: solana_sdk::system_program::id(), // SystemProgram = ER mode
            permission: solana_sdk::system_program::id(),         // dummy for ER mode
        };

        let data = soldex_perps::instruction::DelegatePosition {
            market_id,
            nonce,
            engine_pubkey: self.keeper.pubkey(),
        };

        let ix = Instruction {
            program_id: self.program_id,
            accounts: accounts.to_account_metas(None),
            data: data.data(),
        };

        // Owner is fee payer and must sign client-side — return unsigned
        let msg = Message::new(&[ix], Some(owner));
        let mut tx = Transaction::new_unsigned(msg);
        tx.message.recent_blockhash = recent_blockhash;
        Ok(tx)
    }

    /// Unsigned tx — frontend signs. Send to ER RPC.
    /// Returns position PDA to base layer. Position size must be 0 first.
    pub fn undelegate_position_tx(
        &self,
        owner: &Pubkey,
        position_pda: &Pubkey,
        market_id: [u8; 16],
        nonce: u8,
        recent_blockhash: Hash,
    ) -> Result<Transaction> {
        let accounts = soldex_perps::accounts::UndelegatePosition {
            position: *position_pda,
            owner: *owner,
            magic_context: self.cache.magic_context,
            magic_program: magicblock_program_id(),
        };

        let data = soldex_perps::instruction::UndelegatePosition { market_id, nonce };

        let ix = Instruction {
            program_id: self.program_id,
            accounts: accounts.to_account_metas(None),
            data: data.data(),
        };

        let msg = Message::new(&[ix], Some(owner));
        let mut tx = Transaction::new_unsigned(msg);
        tx.message.recent_blockhash = recent_blockhash;
        Ok(tx)
    }

    // ER layer — keeper-signed, ~10ms. Engine only, never user-facing.

    /// Close a delegated position on ER. Keeper signs, mark_price in 1e6 units.
    /// Send to TEE RPC. PnL calculated on-chain, position wiped after.
    pub fn close_position_er_tx(
        &self,
        position_pda: &Pubkey,
        market_id: [u8; 16],
        nonce: u8,
        mark_price: u64,
        recent_blockhash: Hash,
    ) -> Result<Transaction> {
        // Derive market PDA — same seeds as the program
        let (market_pda, _) =
            Pubkey::find_program_address(&[b"market", &market_id], &self.program_id);

        let accounts = soldex_perps::accounts::ClosePositionEr {
            position: *position_pda,
            owner: self.keeper.pubkey(),
            market: market_pda,
        };
        let data = soldex_perps::instruction::ClosePositionEr {
            mark_price,
            market_id,
            nonce,
        };
        let ix = Instruction {
            program_id: self.program_id,
            accounts: accounts.to_account_metas(None),
            data: data.data(),
        };
        let mut tx = Transaction::new_with_payer(&[ix], Some(&self.keeper.pubkey()));
        tx.sign(&[self.keeper.as_ref()], recent_blockhash);
        Ok(tx)
    }

    /// Liquidate undercollateralised position on ER. Keeper signs. Send to TEE RPC.
    pub fn liquidate_er_tx(
        &self,
        position_pda: &Pubkey,
        market_pda: &Pubkey,
        owner: &Pubkey,
        market_id: [u8; 16],
        nonce: u8,
        mark_price: u64,
        recent_blockhash: Hash,
    ) -> Result<Transaction> {
        let accounts = soldex_perps::accounts::LiquidateEr {
            liquidator: self.keeper.pubkey(),
            position: *position_pda,
            owner: *owner,
            market: *market_pda,
        };

        let data = soldex_perps::instruction::LiquidateEr {
            mark_price,
            market_id,
            nonce,
        };

        let ix = Instruction {
            program_id: self.program_id,
            accounts: accounts.to_account_metas(None),
            data: data.data(),
        };

        let mut tx = Transaction::new_with_payer(&[ix], Some(&self.keeper.pubkey()));
        tx.sign(&[self.keeper.as_ref()], recent_blockhash);
        Ok(tx)
    }

    /// Liquidate position on base layer. Keeper signs. Send to base RPC.
    pub fn liquidate_tx(
        &self,
        position_pda: &Pubkey,
        market_pda: &Pubkey,
        margin_pda: &Pubkey,
        price_feed: &Pubkey,
        recent_blockhash: Hash,
    ) -> Result<Transaction> {
        let accounts = soldex_perps::accounts::Liquidate {
            liquidator: self.keeper.pubkey(),
            market: *market_pda,
            margin: *margin_pda,
            position: *position_pda,
            price_feed: *price_feed,
        };

        let data = soldex_perps::instruction::Liquidate {};

        let ix = Instruction {
            program_id: self.program_id,
            accounts: accounts.to_account_metas(None),
            data: data.data(),
        };

        let mut tx = Transaction::new_with_payer(&[ix], Some(&self.keeper.pubkey()));
        tx.sign(&[self.keeper.as_ref()], recent_blockhash);
        Ok(tx)
    }

    /// Tick funding index on ER. Keeper signs. mark_price in 1e6 units. Send to TEE RPC.
    pub fn funding_tick_er_tx(
        &self,
        market_pda: &Pubkey,
        mark_price: u64,
        recent_blockhash: Hash,
    ) -> Result<Transaction> {
        let accounts = soldex_perps::accounts::FundingTickEr {
            keeper: self.keeper.pubkey(),
            market: *market_pda,
        };

        let data = soldex_perps::instruction::FundingTickEr { mark_price };

        let ix = Instruction {
            program_id: self.program_id,
            accounts: accounts.to_account_metas(None),
            data: data.data(),
        };

        let mut tx = Transaction::new_with_payer(&[ix], Some(&self.keeper.pubkey()));
        tx.sign(&[self.keeper.as_ref()], recent_blockhash);
        Ok(tx)
    }

    /// Delegate market to ER. Call once at startup. Send to base RPC.
    pub fn delegate_market_tx(
        &self,
        market_pda: &Pubkey,
        market_id: [u8; 16],
        recent_blockhash: Hash,
    ) -> Result<Transaction> {
        let (buffer_market, _) =
            Pubkey::find_program_address(&[b"buffer", market_pda.as_ref()], &self.program_id);
        let (delegation_record_market, _) = Pubkey::find_program_address(
            &[b"delegation", market_pda.as_ref()],
            &delegation_program_id(),
        );
        let (delegation_metadata_market, _) = Pubkey::find_program_address(
            &[b"delegation-metadata", market_pda.as_ref()],
            &delegation_program_id(),
        );

        let accounts = soldex_perps::accounts::DelegateMarket {
            market: *market_pda,
            admin: self.keeper.pubkey(),
            buffer_market,
            delegation_record_market,
            delegation_metadata_market,
            owner_program: self.program_id,
            delegation_program: delegation_program_id(),
            system_program: solana_sdk::system_program::id(),
        };

        let data = soldex_perps::instruction::DelegateMarket { market_id };

        let ix = Instruction {
            program_id: self.program_id,
            accounts: accounts.to_account_metas(None),
            data: data.data(),
        };

        let mut tx = Transaction::new_with_payer(&[ix], Some(&self.keeper.pubkey()));
        tx.sign(&[self.keeper.as_ref()], recent_blockhash);
        Ok(tx)
    }

    // ── PDA helpers ───────────────────────────────────────────────────────────

    /// Derive market state PDA — seeds: [b"market", market_id]
    pub fn derive_market_pda(&self, market_id: &[u8; 16]) -> Pubkey {
        Pubkey::find_program_address(&[b"market", market_id.as_ref()], &self.program_id).0
    }

    /// Derive margin account PDA — seeds: [b"margin", market_id, owner]
    pub fn derive_margin_pda(&self, market_id: &[u8; 16], owner: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(
            &[b"margin", market_id.as_ref(), owner.as_ref()],
            &self.program_id,
        )
        .0
    }
}
