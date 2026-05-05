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

    // ── ER LAYER ──────────────────────────────────────────────────────────────
    // Send these to MagicBlock ER RPC. These run at ~10ms.

    // Keeper-signed tx. Send to ER RPC. Runs at 10ms.
    // Called on every price update or trade.
    // NOTE: real PnL/funding logic pending — currently updates timestamp only.

    // TODO: uncomment once program instructions are added
    // pub fn close_position_er_tx(...)  — needs ClosePositionEr in program
    // pub fn funding_tick_er_tx(...)    — needs FundingTickEr in program
    // pub fn liquidate_er_tx(...)       — needs LiquidateEr in program
    // pub fn update_market_er(...)       — needs updatemarket in program
}
