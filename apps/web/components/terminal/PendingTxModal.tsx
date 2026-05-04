'use client'
import React, { useState } from 'react'
import { useMarketStore } from '@/hooks/useMarketStore'
import { useSelfCustodyWallet } from '@/hooks/useSelfCustodyWallet'
import { useEngineWS } from '@/hooks/useEngineWS'
import { Keypair, VersionedTransaction } from '@solana/web3.js'
import { Shield, Zap, X, CheckCircle2, AlertCircle, Loader2 } from 'lucide-react'
import { motion, AnimatePresence } from 'framer-motion'
import bs58 from 'bs58'

export function PendingTxModal() {
  const { pendingTxs } = useMarketStore()
  const { wallet, sign } = useSelfCustodyWallet()
  const { sendMessage } = useEngineWS()
  const [isSigning, setIsSigning] = useState(false)

  if (pendingTxs.length === 0) return null

  const tx = pendingTxs[0]

  const handleSign = async () => {
    if (!wallet) return
    setIsSigning(true)
    try {
      // Decode the transaction
      const txBytes = Buffer.from(tx.txBase64, 'base64')
      const versionedTx = VersionedTransaction.deserialize(txBytes)
      
      // Sign with our self-custody wallet
      const secretKey = bs58.decode(wallet.secretBase58)
      const kp = Keypair.fromSecretKey(secretKey)
      versionedTx.sign([kp])

      // Encode and send back
      const signedBase64 = Buffer.from(versionedTx.serialize()).toString('base64')
      
      sendMessage({
        type: 'submit_transaction',
        tx_base64: signedBase64
      })

      // Remove from store
      useMarketStore.setState((s) => {
        s.pendingTxs = s.pendingTxs.filter(t => t.requestId !== tx.requestId)
      })
    } catch (err) {
      console.error('Signing failed:', err)
      alert('Failed to sign transaction. See console for details.')
    } finally {
      setIsSigning(false)
    }
  }

  const handleDismiss = () => {
    useMarketStore.setState((s) => {
      s.pendingTxs = s.pendingTxs.filter(t => t.requestId !== tx.requestId)
    })
  }

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center p-4 bg-black/80 backdrop-blur-sm">
      <motion.div 
        initial={{ opacity: 0, scale: 0.9, y: 20 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        className="w-full max-w-md bg-[#0a0b0f] border border-[#1e2634] rounded-2xl overflow-hidden shadow-2xl"
      >
        <div className="p-6 border-b border-[#1e2634] flex justify-between items-center bg-[#111827]/50">
          <div className="flex items-center gap-2">
            <div className="p-2 bg-blue-500/10 rounded-lg">
              <Shield className="w-5 h-5 text-blue-400" />
            </div>
            <h3 className="text-lg font-bold text-white uppercase tracking-wider">Sign Transaction</h3>
          </div>
          <button onClick={handleDismiss} className="p-2 hover:bg-[#1e2634] rounded-lg transition-colors">
            <X className="w-5 h-5 text-[#4a5568]" />
          </button>
        </div>

        <div className="p-8 space-y-6">
          <div className="space-y-2">
            <p className="text-[10px] font-bold text-[#4a5568] uppercase tracking-[2px]">Description</p>
            <div className="p-4 bg-[#111827] border border-[#1e2634] rounded-xl">
              <p className="text-sm text-white font-medium leading-relaxed">
                {tx.description}
              </p>
            </div>
          </div>

          <div className="grid grid-cols-2 gap-4">
             <div className="p-3 bg-[#111827] border border-[#1e2634] rounded-xl space-y-1">
                <p className="text-[9px] font-bold text-[#4a5568] uppercase">Network</p>
                <div className="flex items-center gap-1.5">
                   <div className="w-1.5 h-1.5 rounded-full bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.5)]" />
                   <span className="text-xs text-white font-bold">Solana Mainnet</span>
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

          <div className="p-4 bg-blue-500/5 border border-blue-500/10 rounded-xl flex gap-3">
             <AlertCircle className="w-5 h-5 text-blue-400 shrink-0" />
             <p className="text-[11px] text-blue-200/70 leading-relaxed">
                This transaction was built by the Soldex engine and requires your signature to be submitted on-chain. Your private key never leaves your browser.
             </p>
          </div>
        </div>

        <div className="p-6 bg-[#111827]/50 border-t border-[#1e2634] flex gap-3">
          <button 
            onClick={handleDismiss}
            className="flex-1 py-3 px-4 bg-[#1e2634] hover:bg-[#2d3748] text-white text-xs font-bold rounded-xl transition-all uppercase tracking-widest"
          >
            Cancel
          </button>
          <button 
            disabled={isSigning}
            onClick={handleSign}
            className="flex-1 py-3 px-4 bg-blue-500 hover:bg-blue-400 disabled:opacity-50 disabled:cursor-not-allowed text-white text-xs font-bold rounded-xl transition-all shadow-lg shadow-blue-500/20 uppercase tracking-widest flex items-center justify-center gap-2"
          >
            {isSigning ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                Signing...
              </>
            ) : (
              <>
                <Zap className="w-4 h-4" />
                Sign & Submit
              </>
            )}
          </button>
        </div>
      </motion.div>
    </div>
  )
}
