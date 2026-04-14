'use client'
import React from 'react'
import { useMarketStore } from '@/hooks/useMarketStore'

export function TradesPanel({ marketId }: { marketId: string }) {
  const { recentTrades } = useMarketStore()
  return (
    <div className="flex flex-col h-full text-xs font-mono">
      <div className="grid grid-cols-3 px-3 py-1.5 text-[10px] text-[#4b5068] uppercase tracking-wider border-b border-[#1e2130] shrink-0">
        <span>Price</span><span className="text-right">Size</span><span className="text-right">Time</span>
      </div>
      <div className="overflow-auto flex-1">
        {recentTrades.map((t, i) => (
          <div key={i} className="grid grid-cols-3 px-3 py-[3px] hover:bg-[#1a1d28]">
            <span className={`num ${t.side === 'buy' ? 'text-green-400' : 'text-red-400'}`}>{t.price.toFixed(2)}</span>
            <span className="text-right text-[#e2e4ef] num">{t.size.toFixed(3)}</span>
            <span className="text-right text-[#4b5068]">{new Date(t.timestamp).toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' })}</span>
          </div>
        ))}
      </div>
    </div>
  )
}
