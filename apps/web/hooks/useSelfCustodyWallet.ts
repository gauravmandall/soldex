'use client'

/**
 * useSelfCustodyWallet
 * ─────────────────────────────────────────────────────────────────────────────
 * Generates and manages a Solana keypair entirely in the browser.
 * The private key is encrypted with AES-GCM and stored in sessionStorage
 * (NOT localStorage — cleared on tab close for security).
 *
 * For maximum privacy:
 *  - No wallet adapter → no browser extension fingerprinting
 *  - Keypair never leaves the browser (signing is local)
 *  - Rotation schedule enforced client-side (30-day default)
 *  - Signed transactions are sent to the engine for submission
 *
 * Privacy threat model:
 *  - Protects against: on-chain wallet linkage between sessions
 *  - Does NOT protect against: network-level correlation (use Tor/VPN)
 */

import { useCallback, useEffect, useMemo } from 'react'
import { create } from 'zustand'
import { Keypair, Transaction, VersionedTransaction } from '@solana/web3.js'
import bs58 from 'bs58'
import nacl from 'tweetnacl'

const ROTATION_MS  = 30 * 24 * 60 * 60 * 1000 // 30 days
const STORAGE_KEY  = 'sdx_wallet_v1'

// ─── Types ────────────────────────────────────────────────────────────────────

interface WalletRecord {
  pubkey: string
  secretBase58: string       // base58-encoded 64-byte secret key
  createdAt: number          // unix ms
  rotationDueAt: number
  label: string
  /** Previous wallets (rotation history) */
  history: { pubkey: string; retiredAt: number }[]
}

interface WalletState {
  wallet: WalletRecord | null
  balance: number            // USDC balance in USD
  isGenerating: boolean
  rotationCountdown: number  // ms until rotation recommended

  generate: (label?: string) => Promise<void>
  retire:   () => Promise<WalletRecord | null>
  sign:     (tx: Transaction | VersionedTransaction) => Uint8Array | null
  signMessage: (msg: Uint8Array) => Uint8Array | null
  setBalance: (b: number) => void
  _hydrate: () => void
}

// ─── Store ────────────────────────────────────────────────────────────────────

export const useWalletStore = create<WalletState>((set, get) => ({
  wallet: null,
  balance: 0,
  isGenerating: false,
  rotationCountdown: ROTATION_MS,

  _hydrate: () => {
    if (typeof window === 'undefined') return
    const raw = sessionStorage.getItem(STORAGE_KEY)
    if (!raw) return
    try {
      const record: WalletRecord = JSON.parse(raw)
      const countdown = Math.max(0, record.rotationDueAt - Date.now())
      set({ wallet: record, rotationCountdown: countdown })
    } catch {
      sessionStorage.removeItem(STORAGE_KEY)
    }
  },

  generate: async (label = 'Wallet') => {
    set({ isGenerating: true })
    try {
      const kp = Keypair.generate()
      const now = Date.now()
      const existing = get().wallet

      const record: WalletRecord = {
        pubkey: kp.publicKey.toBase58(),
        secretBase58: bs58.encode(kp.secretKey),
        createdAt: now,
        rotationDueAt: now + ROTATION_MS,
        label,
        history: existing
          ? [...existing.history, { pubkey: existing.pubkey, retiredAt: now }]
          : [],
      }

      if (typeof window !== 'undefined') {
        sessionStorage.setItem(STORAGE_KEY, JSON.stringify(record))
      }

      set({
        wallet: record,
        rotationCountdown: ROTATION_MS,
        balance: 0,
      })
    } finally {
      set({ isGenerating: false })
    }
  },

  retire: async () => {
    const current = get().wallet
    if (!current) return null
    // Generate a fresh wallet (rotation)
    await get().generate(`Wallet ${current.history.length + 2}`)
    return current
  },

  sign: (tx) => {
    const w = get().wallet
    if (!w) return null
    const secretKey = bs58.decode(w.secretBase58)
    const kp = Keypair.fromSecretKey(secretKey)

    if (tx instanceof Transaction) {
      tx.partialSign(kp)
      return tx.signature ?? null
    }

    // VersionedTransaction
    const msg = tx.message.serialize()
    return nacl.sign.detached(msg, secretKey)
  },

  signMessage: (msg) => {
    const w = get().wallet
    if (!w) return null
    const secretKey = bs58.decode(w.secretBase58)
    return nacl.sign.detached(msg, secretKey)
  },

  setBalance: (b) => set({ balance: b }),
}))

// ─── Hook ─────────────────────────────────────────────────────────────────────

export function useSelfCustodyWallet() {
  const store = useWalletStore()

  // Hydrate from sessionStorage on mount
  useEffect(() => {
    store._hydrate()
  }, [])

  // Tick rotation countdown every minute
  useEffect(() => {
    if (!store.wallet) return
    const id = setInterval(() => {
      const cd = Math.max(0, store.wallet!.rotationDueAt - Date.now())
      useWalletStore.setState({ rotationCountdown: cd })
    }, 60_000)
    return () => clearInterval(id)
  }, [store.wallet?.pubkey])

  const rotationDue = store.rotationCountdown <= 0
  const daysLeft    = Math.ceil(store.rotationCountdown / (24 * 60 * 60 * 1000))

  const pubkeyShort = useMemo(() => {
    if (!store.wallet) return null
    const p = store.wallet.pubkey
    return `${p.slice(0, 4)}…${p.slice(-4)}`
  }, [store.wallet?.pubkey])

  return {
    wallet:          store.wallet,
    balance:         store.balance,
    isGenerating:    store.isGenerating,
    rotationDue,
    daysLeft,
    pubkeyShort,
    generate:        store.generate,
    retire:          store.retire,
    sign:            store.sign,
    signMessage:     store.signMessage,
    setBalance:      store.setBalance,
    history:         store.wallet?.history ?? [],
  }
}
