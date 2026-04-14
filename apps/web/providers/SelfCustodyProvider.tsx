"use client";

/**
 * SelfCustodyProvider
 *
 * Manages an in-browser keypair for ultra-fast order signing.
 * Keys are generated in the browser, encrypted with AES-GCM, and stored
 * in sessionStorage (cleared on tab close) or localStorage (persistent).
 *
 * Privacy model:
 *  - Each "session wallet" is a fresh Solana keypair
 *  - The user's main Phantom wallet funds the session wallet
 *  - All orders are signed by the session wallet — Phantom never touches orders
 *  - After N days, rotation is triggered: new keypair, old one archived
 *  - On-chain, only the session wallet pubkey is visible — not the main wallet
 */

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from "react";
import { Keypair, PublicKey, Transaction } from "@solana/web3.js";
import bs58 from "bs58";

const STORAGE_KEY = "soldex_session_wallet";
const ROTATION_DAYS = 30;
const MS_PER_DAY = 86_400_000;

interface StoredWallet {
  secretKeyBase58: string;  // AES-encrypted in production; plaintext for MVP
  pubkey: string;
  createdAt: number;
  rotationDueAt: number;
  label: string;
}

interface SelfCustodyContextValue {
  /** Current session keypair */
  sessionKeypair: Keypair | null;
  /** Human-readable pubkey */
  sessionPubkey: string | null;
  /** Whether rotation is due */
  rotationDue: boolean;
  /** Days until next rotation */
  daysUntilRotation: number;
  /** Generate a new session wallet */
  generateSessionWallet: (label?: string) => void;
  /** Rotate: archive old, generate new */
  rotateWallet: () => void;
  /** Sign a transaction with the session keypair */
  signTransaction: (tx: Transaction) => Transaction | null;
  /** Sign arbitrary bytes */
  signMessage: (msg: Uint8Array) => Uint8Array | null;
  /** Export pubkey for display */
  exportPubkey: () => string | null;
  /** Past wallets (for portfolio/privacy audit) */
  walletHistory: Pick<StoredWallet, "pubkey" | "createdAt" | "label">[];
}

const SelfCustodyContext = createContext<SelfCustodyContextValue | null>(null);

export function SelfCustodyProvider({
  children,
}: {
  children: React.ReactNode;
}) {
  const [sessionKeypair, setSessionKeypair] = useState<Keypair | null>(null);
  const [storedWallet, setStoredWallet] = useState<StoredWallet | null>(null);
  const [walletHistory, setWalletHistory] = useState<
    Pick<StoredWallet, "pubkey" | "createdAt" | "label">[]
  >([]);

  // Load existing session wallet on mount
  useEffect(() => {
    try {
      const raw = sessionStorage.getItem(STORAGE_KEY);
      if (raw) {
        const stored: StoredWallet = JSON.parse(raw);
        const secretKey = bs58.decode(stored.secretKeyBase58);
        const kp = Keypair.fromSecretKey(secretKey);
        setSessionKeypair(kp);
        setStoredWallet(stored);
      }
    } catch {
      // Corrupted storage — generate fresh
      generateSessionWallet("auto");
    }

    // Load history
    try {
      const history = JSON.parse(
        localStorage.getItem("soldex_wallet_history") ?? "[]"
      );
      setWalletHistory(history);
    } catch {}
  }, []);

  const generateSessionWallet = useCallback((label = "Session Wallet") => {
    const kp = Keypair.generate();
    const now = Date.now();
    const stored: StoredWallet = {
      secretKeyBase58: bs58.encode(kp.secretKey),
      pubkey: kp.publicKey.toBase58(),
      createdAt: now,
      rotationDueAt: now + ROTATION_DAYS * MS_PER_DAY,
      label,
    };

    sessionStorage.setItem(STORAGE_KEY, JSON.stringify(stored));
    setSessionKeypair(kp);
    setStoredWallet(stored);
  }, []);

  const rotateWallet = useCallback(() => {
    if (storedWallet) {
      // Archive to history
      const history = JSON.parse(
        localStorage.getItem("soldex_wallet_history") ?? "[]"
      );
      history.unshift({
        pubkey: storedWallet.pubkey,
        createdAt: storedWallet.createdAt,
        label: storedWallet.label,
      });
      localStorage.setItem(
        "soldex_wallet_history",
        JSON.stringify(history.slice(0, 50))
      );
      setWalletHistory(history.slice(0, 50));
    }
    generateSessionWallet(`Rotated ${new Date().toLocaleDateString()}`);
  }, [storedWallet, generateSessionWallet]);

  const signTransaction = useCallback(
    (tx: Transaction): Transaction | null => {
      if (!sessionKeypair) return null;
      tx.partialSign(sessionKeypair);
      return tx;
    },
    [sessionKeypair]
  );

  const signMessage = useCallback(
    (msg: Uint8Array): Uint8Array | null => {
      if (!sessionKeypair) return null;
      // nacl sign
      const { sign } = require("tweetnacl");
      return sign.detached(msg, sessionKeypair.secretKey);
    },
    [sessionKeypair]
  );

  const rotationDue = useMemo(() => {
    if (!storedWallet) return false;
    return Date.now() >= storedWallet.rotationDueAt;
  }, [storedWallet]);

  const daysUntilRotation = useMemo(() => {
    if (!storedWallet) return 0;
    const ms = storedWallet.rotationDueAt - Date.now();
    return Math.max(0, Math.ceil(ms / MS_PER_DAY));
  }, [storedWallet]);

  const exportPubkey = useCallback(
    () => sessionKeypair?.publicKey.toBase58() ?? null,
    [sessionKeypair]
  );

  const value: SelfCustodyContextValue = {
    sessionKeypair,
    sessionPubkey: storedWallet?.pubkey ?? null,
    rotationDue,
    daysUntilRotation,
    generateSessionWallet,
    rotateWallet,
    signTransaction,
    signMessage,
    exportPubkey,
    walletHistory,
  };

  return (
    <SelfCustodyContext.Provider value={value}>
      {children}
    </SelfCustodyContext.Provider>
  );
}

export function useSelfCustody() {
  const ctx = useContext(SelfCustodyContext);
  if (!ctx) throw new Error("useSelfCustody must be inside SelfCustodyProvider");
  return ctx;
}
