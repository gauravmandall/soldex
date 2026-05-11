'use client'
import React, { useState } from 'react'
import { useMarketStore } from '@/hooks/useMarketStore'
import { useSelfCustodyWallet } from '@/hooks/useSelfCustodyWallet'
import { useEngineWS } from '@/hooks/useEngineWS'
import { Connection, Keypair, VersionedTransaction } from '@solana/web3.js'
import { Shield, Zap, X, AlertCircle, Loader2 } from 'lucide-react'
import { motion } from 'framer-motion'
import bs58 from 'bs58'
import toast from 'react-hot-toast'

type Status = 'idle' | 'signing' | 'confirming' | 'done' | 'error'

// Step labels shown in the modal header area
const STEP_LABELS: Record<string, string> = {
  deposit:  'Step 1 of 4 — Deposit Collateral',
  base:     'Step 2 of 4 — Init Position',
  delegate: 'Step 3 of 4 — Delegate to MagicBlock',
  er:       'Step 4 of 4 — Open Position (ER)',
}

export function PendingTxModal() {
  const { pendingTxs, pendingOrderParams } = useMarketStore()
  const { wallet } = useSelfCustodyWallet()
  const { sendMessage } = useEngineWS()
  const [status, setStatus] = useState<Status>('idle')

  if (pendingTxs.length === 0) return null

  const tx = pendingTxs[0]
  const isBusy = status === 'signing' || status === 'confirming'
  const stepLabel = STEP_LABELS[tx.submitTo] ?? 'Sign Transaction'

  const handleSign = async () => {
    if (!wallet) return
    setStatus('signing')

   try {
// step 4: send activate_session to engine (no on-chain tx needed)      if (tx.submitTo === 'er') {
        const params = pendingOrderParams
        sendMessage({
          type: 'activate_session',
          request_id: tx.requestId,
          position_pda: tx.positionPda,
          owner_pubkey: tx.ownerPubkey,
          market_id: tx.marketId,
          nonce: tx.nonce,
          size: Math.round((params?.size ?? 0) * 1_000_000),
          entry_price: Math.round(
            (params?.collateral_usdc ?? 0) * (params?.leverage ?? 1) /
            (params?.size ?? 1) * 1_000_000
          ),
          is_long: params?.side === 'long',
          collateral: Math.round((params?.collateral_usdc ?? 0) * 1_000_000),
        })
        useMarketStore.setState((s) => {
          s.pendingTxs = s.pendingTxs.filter(t => t.requestId !== tx.requestId)
          s.pendingOrderParams = null
        })
        setStatus('done')
        toast.success('TEE Session Activated')
        setTimeout(() => setStatus('idle'), 500)
        return
      }

      // ── 1. Decode & sign ──────────────────────────────────────────────────
      const txBytes = Buffer.from(tx.txBase64, 'base64')

      const versionedTx = VersionedTransaction.deserialize(txBytes)
      const kp = Keypair.fromSecretKey(bs58.decode(wallet.secretBase58))
      versionedTx.sign([kp])
      const signedBytes = versionedTx.serialize()

      // ── 2. Submit directly to RPC ─────────────────────────────────────────
      setStatus('confirming')
      const connection = new Connection(process.env.NEXT_PUBLIC_RPC_URL!, 'confirmed')
      const sig = await connection.sendRawTransaction(signedBytes, { skipPreflight: false })

      // ── 3. Wait for on-chain confirmation ─────────────────────────────────
      await connection.confirmTransaction(sig, 'confirmed')

      // validate before removing tx — modal stays open on error
      if (tx.submitTo === 'deposit' && !pendingOrderParams) {
        throw new Error('Missing order params — cannot proceed to build_perps_order')
      }

      // ── 5. Remove this tx from queue ──────────────────────────────────────
      useMarketStore.setState((s) => {
        s.pendingTxs = s.pendingTxs.filter(t => t.requestId !== tx.requestId)
      })

      setStatus('done')
      toast.success(`Confirmed: ${tx.description}`)

      // ── 6. Trigger next step ──────────────────────────────────────────────
     if (tx.submitTo === 'deposit') {
        // pendingOrderParams guaranteed non-null: throw guard above catches it
        const params = pendingOrderParams!
        sendMessage({
          type: 'build_perps_order',
          market_id: params.market_id,
          side: params.side,
          size: params.size,
          leverage: params.leverage,
          collateral_usdc: params.collateral_usdc,
          owner_pubkey: params.owner_pubkey,
        })

      } else if (tx.submitTo === 'base' && tx.positionPda) {
        // initPosition confirmed → delegate to MagicBlock
       sendMessage({
          type: 'delegate_position',
          request_id: tx.requestId,
          position_pda: tx.positionPda,
          owner_pubkey: tx.ownerPubkey,
          market_id: tx.marketId,
          nonce: tx.nonce,
        })
     } else if (tx.submitTo === 'delegate') {
        // Delegation confirmed → queue Step 4 so user sees the final confirmation modal
        useMarketStore.setState((s) => {
          s.pendingTxs.push({
            requestId: tx.requestId + '_activate',
            description: 'Activate TEE Session',
            txBase64: '',        // no on-chain tx — engine message only
            submitTo: 'er',
            positionPda: tx.positionPda,
            ownerPubkey: tx.ownerPubkey,
            marketId: tx.marketId,
            nonce: tx.nonce,
          })
        })
      }

      // Reset status after a beat so next modal opens fresh
      setTimeout(() => setStatus('idle'), 500)

    } catch (err: any) {
      console.error('[PendingTxModal] error:', err)
      setStatus('error')
      toast.error(err?.message ?? 'Transaction failed. See console.')
    }
  }

  const handleDismiss = () => {
    if (isBusy) return // don't allow cancel mid-flight
    useMarketStore.setState((s) => {
      s.pendingTxs = s.pendingTxs.filter(t => t.requestId !== tx.requestId)
    })
    setStatus('idle')
  }

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center p-4 bg-black/80 backdrop-blur-sm">
      <motion.div
        initial={{ opacity: 0, scale: 0.9, y: 20 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        className="w-full max-w-md bg-[#0a0b0f] border border-[#1e2634] rounded-2xl overflow-hidden shadow-2xl"
      >
        {/* Header */}
        <div className="p-6 border-b border-[#1e2634] flex justify-between items-center bg-[#111827]/50">
          <div className="flex items-center gap-2">
            <div className="p-2 bg-blue-500/10 rounded-lg">
              <Shield className="w-5 h-5 text-blue-400" />
            </div>
            <div>
              <h3 className="text-sm font-bold text-white uppercase tracking-wider">Sign Transaction</h3>
              <p className="text-[10px] text-[#4a5568] mt-0.5">{stepLabel}</p>
            </div>
          </div>
          <button
            onClick={handleDismiss}
            disabled={isBusy}
            className="p-2 hover:bg-[#1e2634] rounded-lg transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
          >
            <X className="w-5 h-5 text-[#4a5568]" />
          </button>
        </div>

        <div className="p-8 space-y-6">
          {/* Description */}
          <div className="space-y-2">
            <p className="text-[10px] font-bold text-[#4a5568] uppercase tracking-[2px]">Description</p>
            <div className="p-4 bg-[#111827] border border-[#1e2634] rounded-xl">
              <p className="text-sm text-white font-medium leading-relaxed">{tx.description}</p>
            </div>
          </div>

          {/* Meta grid */}
          <div className="grid grid-cols-2 gap-4">
            <div className="p-3 bg-[#111827] border border-[#1e2634] rounded-xl space-y-1">
              <p className="text-[9px] font-bold text-[#4a5568] uppercase">Network</p>
              <div className="flex items-center gap-1.5">
                <div className="w-1.5 h-1.5 rounded-full bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.5)]" />
                <span className="text-xs text-white font-bold">Solana Devnet</span>
              </div>
            </div>
            <div className="p-3 bg-[#111827] border border-[#1e2634] rounded-xl space-y-1">
              <p className="text-[9px] font-bold text-[#4a5568] uppercase">Security</p>
              <div className="flex items-center gap-1.5">
                <Shield className="w-3 h-3 text-blue-400" />
                <span className="text-xs text-white font-bold">Self-Custody</span>
              </div>
            </div>
          </div>

          {/* Status indicator */}
          {status === 'confirming' && (
            <div className="p-4 bg-amber-500/5 border border-amber-500/20 rounded-xl flex gap-3 items-center">
              <Loader2 className="w-5 h-5 text-amber-400 shrink-0 animate-spin" />
              <p className="text-[11px] text-amber-200/70 leading-relaxed">
                Waiting for on-chain confirmation…
              </p>
            </div>
          )}

          {status === 'error' && (
            <div className="p-4 bg-red-500/5 border border-red-500/20 rounded-xl flex gap-3 items-center">
              <AlertCircle className="w-5 h-5 text-red-400 shrink-0" />
              <p className="text-[11px] text-red-200/70 leading-relaxed">
                Transaction failed. Check console for details.
              </p>
            </div>
          )}

          {status === 'idle' && (
            <div className="p-4 bg-blue-500/5 border border-blue-500/10 rounded-xl flex gap-3">
              <AlertCircle className="w-5 h-5 text-blue-400 shrink-0" />
              <p className="text-[11px] text-blue-200/70 leading-relaxed">
                This transaction was built by the Soldex engine and requires your signature. Your private key never leaves your browser.
              </p>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-6 bg-[#111827]/50 border-t border-[#1e2634] flex gap-3">
          <button
            onClick={handleDismiss}
            disabled={isBusy}
            className="flex-1 py-3 px-4 bg-[#1e2634] hover:bg-[#2d3748] disabled:opacity-30 disabled:cursor-not-allowed text-white text-xs font-bold rounded-xl transition-all uppercase tracking-widest"
          >
            Cancel
          </button>
          <button
            disabled={isBusy}
            onClick={handleSign}
            className="flex-1 py-3 px-4 bg-blue-500 hover:bg-blue-400 disabled:opacity-50 disabled:cursor-not-allowed text-white text-xs font-bold rounded-xl transition-all shadow-lg shadow-blue-500/20 uppercase tracking-widest flex items-center justify-center gap-2"
          >
            {status === 'signing' ? (
              <><Loader2 className="w-4 h-4 animate-spin" /> Signing…</>
            ) : status === 'confirming' ? (
              <><Loader2 className="w-4 h-4 animate-spin" /> Confirming…</>
            ) : (
              <><Zap className="w-4 h-4" /> Sign & Submit</>
            )}
          </button>
        </div>
      </motion.div>
    </div>
  )
}