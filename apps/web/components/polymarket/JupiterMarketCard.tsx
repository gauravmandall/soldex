'use client'
import React from 'react'
import { JupiterOpportunity } from '@/hooks/useMarketStore'
import { formatVolume } from '@/lib/format'
import { motion } from 'framer-motion'
import { Zap, ChevronRight, Activity } from 'lucide-react'

interface Props {
  market: JupiterOpportunity
  onClick: () => void
  active?: boolean
}

export function JupiterMarketCard({ market, onClick, active = false }: Props) {
  const yesPrice = (market.outcome_prices[0] * 100).toFixed(0)
  const noPrice = (market.outcome_prices[1] * 100).toFixed(0)

  return (
    <motion.div 
      layout
      onClick={onClick}
      className={`p-0 rounded-2xl border transition-all cursor-pointer group flex flex-col overflow-hidden ${
        active 
          ? 'bg-[#1e2634] border-blue-500/50 shadow-[0_0_30px_rgba(59,130,246,0.15)]' 
          : 'bg-[#0d1117] border-[#1e2634] hover:border-[#3b82f6]/40 hover:bg-[#111827]'
      }`}
    >
      {/* Thumbnail Area */}
      <div className="relative h-32 w-full bg-[#1a1f2e] overflow-hidden">
        {market.image_url ? (
          <img 
            src={market.image_url} 
            alt={market.title}
            className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-500"
          />
        ) : (
          <div className="w-full h-full flex items-center justify-center">
             <Zap className="w-8 h-8 text-blue-500/20" />
          </div>
        )}
        <div className="absolute top-3 left-3 flex items-center gap-2">
            <div className="px-2 py-1 bg-black/60 backdrop-blur-md rounded-md border border-white/10 flex items-center gap-1.5">
               <div className="w-1.5 h-1.5 rounded-full bg-blue-500 animate-pulse" />
               <span className="text-[9px] font-bold text-white uppercase tracking-widest">{market.category}</span>
            </div>
        </div>
      </div>

      <div className="p-5 flex flex-col gap-4">
        <h3 className="text-[14px] font-bold text-white leading-snug line-clamp-2 h-10 group-hover:text-blue-400 transition-colors">
          {market.title}
        </h3>
        
        <div className="flex items-center justify-between text-[11px] font-bold">
           <div className="flex flex-col">
              <span className="text-[#4a5568] uppercase text-[9px] tracking-tighter">Volume 24H</span>
              <span className="text-white">{formatVolume(market.volume_24h)}</span>
           </div>
           <div className="flex items-center gap-3">
              <div className="flex flex-col items-end">
                 <span className="text-[#4a5568] uppercase text-[9px] tracking-tighter">Yes</span>
                 <span className="text-emerald-500">{yesPrice}¢</span>
              </div>
              <div className="flex flex-col items-end">
                 <span className="text-[#4a5568] uppercase text-[9px] tracking-tighter">No</span>
                 <span className="text-rose-500">{noPrice}¢</span>
              </div>
           </div>
        </div>

        <div className="flex gap-2 pt-1">
           <button 
             onClick={(e) => { e.stopPropagation(); onClick(); }}
             className="flex-1 py-2 bg-emerald-500/10 border border-emerald-500/20 rounded-lg text-[10px] font-bold text-emerald-500 hover:bg-emerald-500 hover:text-black transition-all uppercase tracking-widest"
           >
             BUY YES
           </button>
           <button 
             onClick={(e) => { e.stopPropagation(); onClick(); }}
             className="flex-1 py-2 bg-rose-500/10 border border-rose-500/20 rounded-lg text-[10px] font-bold text-rose-500 hover:bg-rose-500 hover:text-white transition-all uppercase tracking-widest"
           >
             BUY NO
           </button>
        </div>
      </div>
    </motion.div>
  )
}
