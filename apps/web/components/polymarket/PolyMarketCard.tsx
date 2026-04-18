'use client'
import React from 'react'
import { PolyOpportunity } from '@/hooks/useMarketStore'
import { formatVolume, formatUsd } from '@/lib/format'
import { motion } from 'framer-motion'
import { TrendingUp, ChevronRight } from 'lucide-react'

interface Props {
  market: PolyOpportunity
  onClick: () => void
  active?: boolean
}

export function PolyMarketCard({ market, onClick, active = false }: Props) {
  const yesProb = Math.round(market.yes_price * 100)
  const noProb = 100 - yesProb

  return (
    <div 
      onClick={onClick}
      className={`px-4 py-3 border-b border-[var(--bd)] cursor-pointer transition-all flex items-center justify-between font-mono group ${
        active ? 'bg-[var(--bg2)] border-l-2 border-l-[var(--amber)]' : 'hover:bg-[var(--bg1)]'
      }`}
    >
      <div className="flex flex-col gap-1 flex-1 min-w-0 pr-8">
        <h3 className="text-[12px] font-bold text-[var(--tx)] truncate group-hover:text-[var(--amber)] transition-colors uppercase tracking-tight">
          {market.question}
        </h3>
        <div className="flex items-center gap-4 text-[10px] font-medium uppercase tracking-tight">
           <span className="px-1.5 py-0.5 bg-[var(--bd2)] text-[var(--tx2)] rounded-[2px]">{market.category}</span>
           <span className="text-[var(--tx3)]">VOL: <span className="text-[var(--tx)]">{formatVolume(market.volume_24h)}</span></span>
           <span className="text-[var(--tx3)]">LIQ: <span className="text-[var(--tx)]">{formatUsd(market.liquidity)}</span></span>
        </div>
      </div>

      <div className="flex items-center gap-6 shrink-0">
        {/* Probability Column */}
        <div className="flex flex-col items-end gap-1 w-[80px]">
           <div className="flex justify-between w-full text-[11px] font-bold">
              <span className="text-[var(--green)]">{yesProb}¢</span>
              <span className="text-[var(--red)]">{noProb}¢</span>
           </div>
           <div className="w-full h-[3px] bg-[var(--bd)] rounded-full overflow-hidden flex">
              <div 
                className="h-full bg-[var(--green)] transition-all duration-500" 
                style={{ width: `${yesProb}%` }} 
              />
           </div>
        </div>
        <ChevronRight className={`w-4 h-4 transition-all ${active ? 'text-[var(--amber)] translate-x-1' : 'text-[var(--tx3)] group-hover:text-[var(--tx2)]'}`} />
      </div>
    </div>
  )
}
