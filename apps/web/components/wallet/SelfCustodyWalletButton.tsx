'use client'
import React, { useState } from 'react'
import { useSelfCustodyWallet } from '@/hooks/useSelfCustodyWallet'
import { Shield, RefreshCw, Copy, ChevronDown, AlertTriangle } from 'lucide-react'
import toast from 'react-hot-toast'

export function SelfCustodyWalletButton() {
  const { wallet, pubkeyShort, isGenerating, rotationDue, daysLeft, generate, retire, balance } = useSelfCustodyWallet()
  const [open, setOpen] = useState(false)

  const copyPubkey = () => {
    if (!wallet) return
    navigator.clipboard.writeText(wallet.pubkey)
    toast.success('Address copied')
  }

  if (!wallet) return (
    <button onClick={() => generate()} disabled={isGenerating}
      className="flex items-center gap-1.5 px-3 py-1.5 bg-blue-600 hover:bg-blue-500 text-white text-xs rounded transition-colors disabled:opacity-50">
      <Shield className="w-3 h-3" />
      {isGenerating ? 'Generating…' : 'New Wallet'}
    </button>
  )

  return (
    <div className="relative">
      <button onClick={() => setOpen(!open)}
        className={`flex items-center gap-1.5 px-3 py-1.5 text-xs rounded border transition-colors ${rotationDue ? 'bg-yellow-500/10 border-yellow-500/40 text-yellow-400' : 'bg-[#13151d] border-[#1e2130] text-[#e2e4ef] hover:border-blue-500/40'}`}>
        {rotationDue && <AlertTriangle className="w-3 h-3" />}
        <Shield className="w-3 h-3 text-blue-400" />
        <span className="font-mono">{pubkeyShort}</span>
        <span className="text-[#4b5068]">·</span>
        <span className="num">${balance.toFixed(2)}</span>
        <ChevronDown className={`w-3 h-3 transition-transform ${open ? 'rotate-180' : ''}`} />
      </button>
      {open && (
        <div className="absolute right-0 top-full mt-1 w-72 bg-[#13151d] border border-[#1e2130] rounded shadow-2xl z-50 p-3 flex flex-col gap-2">
          <div className="flex items-center justify-between">
            <span className="text-xs text-[#4b5068]">Self-Custody Wallet</span>
            <span className="text-[10px] text-green-400 flex items-center gap-1"><span className="w-1.5 h-1.5 rounded-full bg-green-400" /> Local</span>
          </div>
          <div className="flex items-center justify-between bg-[#0a0b0f] rounded p-2 border border-[#1e2130]">
            <span className="font-mono text-[10px] text-[#8b90a8] break-all">{wallet.pubkey.slice(0, 24)}…</span>
            <button onClick={copyPubkey} className="text-[#4b5068] hover:text-white ml-2"><Copy className="w-3 h-3" /></button>
          </div>
          <div className="flex justify-between text-xs">
            <span className="text-[#4b5068]">Balance</span>
            <span className="text-white num">${balance.toFixed(2)} USDC</span>
          </div>
          <div className="flex justify-between text-xs">
            <span className="text-[#4b5068]">Privacy Rotation</span>
            <span className={rotationDue ? 'text-yellow-400' : 'text-green-400'}>{rotationDue ? 'Due now!' : `${daysLeft}d left`}</span>
          </div>
          <div className="border-t border-[#1e2130] pt-2 flex gap-2">
            <button onClick={() => { retire(); setOpen(false) }}
              className="flex-1 flex items-center justify-center gap-1 py-1.5 text-xs border border-yellow-500/30 text-yellow-400 rounded hover:bg-yellow-500/10">
              <RefreshCw className="w-3 h-3" /> Rotate
            </button>
            <button onClick={() => { generate(); setOpen(false) }}
              className="flex-1 py-1.5 text-xs border border-[#1e2130] text-[#8b90a8] rounded hover:text-white">
              New
            </button>
          </div>
          <p className="text-[10px] text-[#4b5068]">🔒 Private key in session only. Rotation unlinks your on-chain history.</p>
        </div>
      )}
    </div>
  )
}
