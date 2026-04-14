'use client'
import React, { useState } from 'react'
import { useMarketStore } from '@/hooks/useMarketStore'
import { ChevronDown } from 'lucide-react'

const MARKETS = ['SOL-PERP', 'BTC-PERP', 'ETH-PERP']

export function MarketSelector() {
  const { activeMarket, setActiveMarket } = useMarketStore()
  const [open, setOpen] = useState(false)
  return (
    <div className="relative">
      <button onClick={() => setOpen(!open)}
        className="flex items-center gap-1.5 text-sm font-bold text-white hover:text-blue-300 transition-colors">
        {activeMarket}
        <ChevronDown className={`w-3.5 h-3.5 transition-transform ${open ? 'rotate-180' : ''}`} />
      </button>
      {open && (
        <div className="absolute top-full left-0 mt-1 bg-[#13151d] border border-[#1e2130] rounded shadow-2xl z-50 min-w-[140px]">
          {MARKETS.map(m => (
            <button key={m} onClick={() => { setActiveMarket(m); setOpen(false) }}
              className={`w-full px-3 py-2 text-left text-xs hover:bg-[#1e2130] transition-colors ${m === activeMarket ? 'text-blue-400' : 'text-[#e2e4ef]'}`}>
              {m}
            </button>
          ))}
        </div>
      )}
    </div>
  )
}
