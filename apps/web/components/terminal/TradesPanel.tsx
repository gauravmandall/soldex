'use client'
import React from 'react'
import { useMarketStore } from '@/hooks/useMarketStore'

export function TradesPanel({ marketId }: { marketId: string }) {
  const { recentTrades } = useMarketStore()
  return (
    <div className="flex flex-col h-full text-xs font-mono bg-[#0a0e15]">
      <div className="grid grid-cols-3 px-3 py-2 text-[10px] font-bold text-[#4a5568] uppercase tracking-wider border-b border-[#1e2634] shrink-0">
        <span>Price</span><span className="text-right">Size</span><span className="text-right">Time</span>
      </div>
      <div className="overflow-auto flex-1 no-scrollbar">
        {recentTrades.map((t, i) => (
          <div key={i} className="grid grid-cols-3 px-3 py-[4px] hover:bg-[#1c212e] transition-colors border-b border-white/[0.02]">
            <span className={`num font-bold ${t.side === 'buy' ? 'text-[#10b981]' : 'text-[#ef4444]'}`}>
              {t.price.toLocaleString(undefined, { minimumFractionDigits: 3 })}
            </span>
            <span className="text-right text-[#94a3b8] font-medium num">{t.size.toFixed(2)}</span>
            <span className="text-right text-[#4a5568] text-[10px] font-medium">
              {new Date(t.timestamp).toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' })}
            </span>
          </div>
        ))}
      </div>
    </div>
  )
}
