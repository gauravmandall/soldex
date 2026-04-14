/**
 * Self-custody wallet utilities for the SDK.
 * Frontend-side: keypair generation, signing, rotation management.
 */
import { Keypair, Transaction, VersionedTransaction } from '@solana/web3.js'
import bs58 from 'bs58'

export const ROTATION_DAYS = 30

export interface WalletRotationState {
  pubkey: string
  secretBase58: string
  createdAt: number
  rotationDueAt: number
  history: Array<{ pubkey: string; retiredAt: number }>
}

/** Generate a fresh Solana keypair */
export function generateKeypair(): WalletRotationState {
  const kp = Keypair.generate()
  const now = Date.now()
  return {
    pubkey: kp.publicKey.toBase58(),
    secretBase58: bs58.encode(kp.secretKey),
    createdAt: now,
    rotationDueAt: now + ROTATION_DAYS * 86_400_000,
    history: [],
  }
}

/** Sign a transaction with a stored keypair */
export function signTx(
  tx: Transaction,
  state: WalletRotationState,
): Transaction {
  const kp = Keypair.fromSecretKey(bs58.decode(state.secretBase58))
  tx.partialSign(kp)
  return tx
}

/** Check if rotation is due */
export function isRotationDue(state: WalletRotationState): boolean {
  return Date.now() >= state.rotationDueAt
}

/** Rotate: retire current keypair, generate new one */
export function rotateWallet(state: WalletRotationState): WalletRotationState {
  const now = Date.now()
  const fresh = generateKeypair()
  return {
    ...fresh,
    history: [
      ...state.history,
      { pubkey: state.pubkey, retiredAt: now },
    ],
  }
}
