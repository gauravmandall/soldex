'use client'
import React, { useState } from 'react'
import { useMarketStore } from '@/hooks/useMarketStore'
import { ChevronDown } from 'lucide-react'

const MARKETS = ['SOL-USDC', 'BTC-USDC', 'ETH-USDC', 'JUP-USDC', 'WIF-USDC', 'PYTH-USDC']

export function MarketSelector() {
  const { activeMarket, setActiveMarket } = useMarketStore()
  const [open, setOpen] = useState(false)

  return (
    <div className="relative">
      <button onClick={() => setOpen(!open)}
        className="flex items-center gap-1.5 transition-all text-[#e2e8f0] hover:text-[#3b82f6] group">
        <span className="text-sm font-bold tracking-tight">{activeMarket}</span>
        <ChevronDown className={`w-3.5 h-3.5 transition-transform duration-200 ${open ? 'rotate-180' : ''} text-[#4a5568] group-hover:text-[#3b82f6]`} />
      </button>

      {open && (
        <>
          <div className="fixed inset-0 z-40" onClick={() => setOpen(false)} />
          <div className="absolute top-full left-0 mt-2 bg-[#111827] border border-[#1e2634] rounded-lg shadow-[0_8px_32px_rgba(0,0,0,0.5)] z-50 min-w-[180px] py-1 overflow-hidden animate-in fade-in slide-in-from-top-1 duration-150">
            <div className="px-3 py-1.5 text-[9px] font-bold text-[#4a5568] uppercase tracking-widest border-b border-[#1e2634] mb-1">
              Popular Markets
            </div>
            {MARKETS.map(m => (
              <button key={m} onClick={() => { setActiveMarket(m); setOpen(false) }}
                className={`w-full px-4 py-2 text-left text-[11px] font-mono transition-colors flex items-center justify-between ${
                  m === activeMarket ? 'bg-[#1e3a5f] text-[#60a5fa]' : 'text-[#94a3b8] hover:bg-[#1c212e] hover:text-[#e2e8f0]'
                }`}>
                {m}
                {m === activeMarket && <div className="w-1.5 h-1.5 rounded-full bg-[#3b82f6] shadow-[0_0_8px_rgba(59,130,246,0.5)]" />}
              </button>
            ))}
          </div>
        </>
      )}
    </div>
  )
}
