'use client'
import React from 'react'
import { PolyOpportunity } from '@/hooks/useMarketStore'
import { formatVolume, formatUsd } from '@/lib/format'
import { motion } from 'framer-motion'
import { TrendingUp, ChevronRight, Activity } from 'lucide-react'

interface Props {
  market: PolyOpportunity
  onClick: () => void
  active?: boolean
}

export function PolyMarketCard({ market, onClick, active = false }: Props) {
  const yesPrice = (market.yes_price * 100).toFixed(0)
  const noPrice = (market.no_price * 100).toFixed(0)

  return (
    <motion.div 
      layout
      onClick={onClick}
      className={`p-5 rounded-xl border transition-all cursor-pointer group flex flex-col gap-4 ${
        active 
          ? 'bg-[#1e2634] border-blue-500/50 shadow-[0_0_20px_rgba(59,130,246,0.1)]' 
          : 'bg-[#0d1117] border-[#1e2634] hover:border-[#3b82f6]/40 hover:bg-[#111827]'
      }`}
    >
      <div className="flex items-center justify-between">
         <div className="flex items-center gap-2">
            <div className="w-8 h-8 rounded-full bg-blue-500/10 border border-blue-500/20 flex items-center justify-center">
               <Activity className="w-4 h-4 text-blue-500" />
            </div>
            <span className="text-[10px] font-bold text-[#4a5568] uppercase tracking-widest">{market.category}</span>
         </div>
         <TrendingUp className="w-3.5 h-3.5 text-[#4a5568] group-hover:text-blue-500 transition-colors" />
      </div>

      <div className="flex flex-col gap-3">
        <h3 className="text-[14px] font-bold text-white leading-snug line-clamp-2 h-10 group-hover:text-blue-400 transition-colors">
          {market.question}
        </h3>
        
        <div className="flex items-center justify-between text-[11px] font-bold mt-2">
           <div className="flex flex-col">
              <span className="text-[#4a5568] uppercase text-[9px] tracking-tighter">Volume 24H</span>
              <span className="text-white">{formatVolume(market.volume_24h)}</span>
           </div>
           <div className="flex items-center gap-4">
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

        <div className="flex gap-2 pt-2">
           <button 
             onClick={(e) => { e.stopPropagation(); onClick(); }}
             className="flex-1 py-2 bg-emerald-500/10 border border-emerald-500/20 rounded-lg text-[11px] font-bold text-emerald-500 hover:bg-emerald-500 hover:text-black transition-all uppercase tracking-widest"
           >
             Yes {yesPrice}¢
           </button>
           <button 
             onClick={(e) => { e.stopPropagation(); onClick(); }}
             className="flex-1 py-2 bg-rose-500/10 border border-rose-500/20 rounded-lg text-[11px] font-bold text-rose-500 hover:bg-rose-500 hover:text-white transition-all uppercase tracking-widest"
           >
             No {noPrice}¢
           </button>
        </div>
      </div>
    </motion.div>
  )
}
