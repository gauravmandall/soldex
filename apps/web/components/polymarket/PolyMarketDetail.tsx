'use client'
import React, { useState, useEffect, useMemo } from 'react'
import { PolyOpportunity, JupiterOpportunity, useMarketStore } from '@/hooks/useMarketStore'
import { PolyChart } from './PolyChart'
import { formatUsd, formatVolume } from '@/lib/format'
import { motion, AnimatePresence } from 'framer-motion'
import { 
  X, ExternalLink, ShieldCheck, Info, TrendingUp, 
  BarChart3, Activity, Zap, Clock, Maximize2,
  ChevronRight
} from 'lucide-react'
import { useEngineWS } from '@/hooks/useEngineWS'
import { useSelfCustodyWallet } from '@/hooks/useSelfCustodyWallet'

interface Props {
  market: PolyOpportunity | JupiterOpportunity
  onBack: () => void
}

export function PolyMarketDetail({ market, onBack }: Props) {
  const [outcomeIndex, setOutcomeIndex] = useState(0)
  const [orderType, setOrderType] = useState<'market' | 'limit'>('market')
  const [amount, setAmount] = useState('100')
  const { polyMarketHistory, polyOrderbooks, polyTrades } = useMarketStore()
  const { sendMessage } = useEngineWS()
  const { wallet } = useSelfCustodyWallet()
  const [chartData, setChartData] = useState<{time: number, value: number}[]>([])
  
  const isJupiter = market.source === 'jupiter'
  const title = 'question' in market ? market.question : market.title
  
  const yesPrice = isJupiter ? (market as JupiterOpportunity).outcome_prices[0] : (market as PolyOpportunity).yes_price
  const noPrice = isJupiter ? (market as JupiterOpportunity).outcome_prices[1] : (market as PolyOpportunity).no_price
  const outcomes = 'outcomes' in market ? market.outcomes : ['YES', 'NO']

  useEffect(() => {
    // History generation logic...
    const now = Math.floor(Date.now() / 1000)
    const data = []
    let lastVal = yesPrice
    const points = 48
    for (let i = 0; i < points; i++) {
      const time = now - (points - i) * 3600
      lastVal = Math.max(0.01, Math.min(0.99, lastVal + (Math.random() - 0.48) * 0.02))
      data.push({ time, value: lastVal })
    }
    setChartData(data)
  }, [market, yesPrice])

  const prob = outcomeIndex === 0 ? yesPrice : noPrice
  const potentialReturn = amount ? ((parseFloat(amount) / prob) - parseFloat(amount)).toFixed(2) : "—"

  return (
    <motion.div 
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      className="flex flex-col lg:flex-row gap-8 pb-20"
    >
      {/* MAIN CONTENT (LEFT/CENTER) */}
      <div className="flex-1 flex flex-col gap-8 min-w-0">
        {/* HEADER AREA */}
        <div className="flex flex-col gap-4">
           <button onClick={onBack} className="flex items-center gap-2 text-[#4a5568] hover:text-white transition-colors text-[11px] font-bold uppercase tracking-widest w-fit">
              <ChevronRight className="w-4 h-4 rotate-180" />
              Back to Markets
           </button>
           
            <div className="flex items-start justify-between">
               <div className="flex items-start gap-6">
                  {market.image_url && (
                     <div className="w-24 h-24 rounded-2xl overflow-hidden shrink-0 border border-[#1e2634]">
                        <img src={market.image_url} alt={title} className="w-full h-full object-cover" />
                     </div>
                  )}
                  <div className="flex flex-col gap-3">
                     <div className="flex items-center gap-3">
                        <span className="px-2 py-0.5 bg-[#1e2634] text-[#4a5568] text-[10px] font-bold rounded-md uppercase tracking-wider">{market.category}</span>
                        <span className="text-[#4a5568] text-[11px]">•</span>
                        <span className="text-[#4a5568] text-[11px] uppercase font-bold tracking-tighter">Volume: {formatVolume(market.volume_24h)}</span>
                     </div>
                     <h1 className="text-3xl font-bold text-white leading-tight tracking-tight max-w-[800px]">
                        {title}
                     </h1>
                  </div>
               </div>
               <div className="flex items-center gap-3">
                  <button className="p-2.5 bg-[#111827] border border-[#1e2634] rounded-xl text-[#4a5568] hover:text-white transition-all"><Maximize2 className="w-4 h-4" /></button>
                  <button className="p-2.5 bg-[#111827] border border-[#1e2634] rounded-xl text-[#4a5568] hover:text-white transition-all"><ExternalLink className="w-4 h-4" /></button>
               </div>
            </div>
        </div>

        {/* CHART & OUTCOMES GRID */}
        <div className="grid grid-cols-1 gap-8">
           <div className="bg-[#0d1117] border border-[#1e2634] rounded-2xl overflow-hidden">
              <div className="h-[300px] w-full p-4">
                 <PolyChart data={chartData} color={isJupiter ? "#3b82f6" : "#10b981"} />
              </div>
           </div>

           <div className="flex flex-col gap-4">
              <h3 className="text-sm font-bold text-white uppercase tracking-widest">Outcomes</h3>
              <div className="flex flex-col gap-2">
                 {[
                   { label: outcomes[0], price: yesPrice, color: 'emerald', index: 0 },
                   { label: outcomes[1], price: noPrice, color: 'rose', index: 1 }
                 ].map((out) => (
                    <div key={out.index} className="flex items-center justify-between p-4 bg-[#0d1117] border border-[#1e2634] rounded-xl hover:border-[#3b82f6]/40 transition-all group">
                       <div className="flex items-center gap-4">
                          <div className={`w-2 h-2 rounded-full ${out.index === 0 ? 'bg-emerald-500' : 'bg-rose-500'}`} />
                          <span className="font-bold text-white uppercase tracking-wider">{out.label}</span>
                       </div>
                       <div className="flex items-center gap-8">
                          <span className="text-xl font-bold text-white">{(out.price * 100).toFixed(0)}¢</span>
                          <div className="flex gap-2">
                             <button 
                               onClick={() => { setOutcomeIndex(out.index); }}
                               className={`px-6 py-2 rounded-lg text-[11px] font-bold uppercase tracking-widest transition-all ${
                                  out.index === 0 
                                    ? 'bg-emerald-500/10 text-emerald-500 border border-emerald-500/20 hover:bg-emerald-500 hover:text-black' 
                                    : 'bg-rose-500/10 text-rose-500 border border-rose-500/20 hover:bg-rose-500 hover:text-white'
                               }`}
                             >
                                Buy {out.label}
                             </button>
                          </div>
                       </div>
                    </div>
                 ))}
              </div>
           </div>
        </div>
      </div>

      {/* ORDER PANEL (RIGHT) */}
      <div className="w-full lg:w-[400px] shrink-0">
         <div className="sticky top-8 bg-[#0d1117] border border-[#1e2634] rounded-2xl overflow-hidden flex flex-col shadow-2xl">
            <div className="p-6 border-b border-[#1e2634] flex flex-col gap-6">
               <div className="flex items-center justify-between">
                  <div className="flex gap-4">
                     <button className="text-sm font-bold text-white border-b-2 border-blue-500 pb-1">Buy</button>
                     <button className="text-sm font-bold text-[#4a5568] hover:text-white transition-colors pb-1">Sell</button>
                  </div>
                  <div className="flex bg-[#111827] rounded-lg p-1 border border-[#1e2634]">
                     {['market', 'limit'].map(t => (
                       <button 
                         key={t}
                         onClick={() => setOrderType(t as any)}
                         className={`px-3 py-1 text-[10px] font-bold uppercase rounded-md transition-all ${
                           orderType === t ? 'bg-[#1e2634] text-white' : 'text-[#4a5568]'
                         }`}
                       >
                         {t}
                       </button>
                     ))}
                  </div>
               </div>

               <div className="flex gap-2">
                  <button 
                    onClick={() => setOutcomeIndex(0)}
                    className={`flex-1 py-3 rounded-xl border font-bold text-xs uppercase tracking-widest transition-all ${
                      outcomeIndex === 0 ? 'bg-emerald-500/10 border-emerald-500 text-emerald-500' : 'bg-[#111827] border-[#1e2634] text-[#4a5568]'
                    }`}
                  >
                    {outcomes[0]} {(yesPrice * 100).toFixed(0)}¢
                  </button>
                  <button 
                    onClick={() => setOutcomeIndex(1)}
                    className={`flex-1 py-3 rounded-xl border font-bold text-xs uppercase tracking-widest transition-all ${
                      outcomeIndex === 1 ? 'bg-rose-500/10 border-rose-500 text-rose-500' : 'bg-[#111827] border-[#1e2634] text-[#4a5568]'
                    }`}
                  >
                    {outcomes[1]} {(noPrice * 100).toFixed(0)}¢
                  </button>
               </div>
            </div>

            <div className="p-6 flex flex-col gap-6">
               <div className="flex flex-col gap-2">
                  <div className="flex justify-between text-[11px] font-bold uppercase tracking-wider text-[#4a5568]">
                     <span>Amount</span>
                     <span>$0.00 cash</span>
                  </div>
                  <div className="relative">
                     <input 
                       type="number"
                       value={amount}
                       onChange={e => setAmount(e.target.value)}
                       className="w-full bg-[#111827] border border-[#1e2634] rounded-xl px-4 py-4 text-2xl font-bold text-white outline-none focus:border-blue-500 transition-all pr-12"
                     />
                     <span className="absolute right-4 top-1/2 -translate-y-1/2 font-bold text-[#4a5568]">$</span>
                  </div>
                  <div className="flex gap-2">
                     {['10', '50', '100', '500'].map(v => (
                       <button key={v} onClick={() => setAmount(v)} className="flex-1 py-1.5 bg-[#111827] border border-[#1e2634] rounded-lg text-[10px] font-bold text-[#4a5568] hover:text-white hover:border-[#3b82f6]/40 transition-all">
                          +${v}
                       </button>
                     ))}
                  </div>
               </div>

               <div className="flex flex-col gap-3 py-4 border-y border-[#1e2634]/50">
                  <div className="flex justify-between text-[11px]">
                     <span className="text-[#4a5568] uppercase font-bold tracking-tight">Est. Shares</span>
                     <span className="text-white font-bold">{amount ? (parseFloat(amount) / prob).toFixed(1) : '0.0'}</span>
                  </div>
                  <div className="flex justify-between text-[11px]">
                     <span className="text-[#4a5568] uppercase font-bold tracking-tight">Max Profit</span>
                     <span className="text-emerald-500 font-bold">${potentialReturn}</span>
                  </div>
               </div>

                <button 
                  onClick={() => {
                    if (!wallet) {
                        alert('Please connect your self-custody wallet first');
                        return;
                    }

                    if (isJupiter) {
                        sendMessage({
                            type: 'jupiter_prediction_order',
                            market_id: market.market_id,
                            outcome_index: outcomeIndex,
                            size_usdc: parseFloat(amount),
                            owner_pubkey: wallet.pubkey
                        })
                    }
                  }}
                  className={`w-full py-4 rounded-xl text-sm font-bold uppercase tracking-[2px] transition-all active:scale-95 shadow-xl ${
                  outcomeIndex === 0 ? 'bg-emerald-500 text-black hover:bg-emerald-400' : 'bg-rose-500 text-white hover:bg-rose-400'
                }`}>
                  {wallet ? 'Confirm Execution' : 'Connect Wallet'}
                </button>
               
               <p className="text-[10px] text-[#4a5568] text-center px-4 leading-relaxed">
                  By trading, you agree to the <span className="text-blue-500 hover:underline cursor-pointer">Terms of Use</span> and realize markets are for entertainment.
               </p>
            </div>
         </div>
      </div>
    </motion.div>
  )
}
