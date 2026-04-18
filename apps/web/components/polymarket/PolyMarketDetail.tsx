'use client'
import React, { useState, useEffect, useMemo } from 'react'
import { PolyOpportunity, useMarketStore } from '@/hooks/useMarketStore'
import { PolyChart } from './PolyChart'
import { formatUsd, formatVolume } from '@/lib/format'
import { motion, AnimatePresence } from 'framer-motion'
import { 
  X, ExternalLink, ShieldCheck, Info, TrendingUp, 
  BarChart3, Activity, Zap, Clock, Maximize2
} from 'lucide-react'

interface Props {
  market: PolyOpportunity
  onBack: () => void
}

export function PolyMarketDetail({ market, onBack }: Props) {
  const [side, setSide] = useState<'YES' | 'NO'>('YES')
  const [amount, setAmount] = useState('100')
  const { polyMarketHistory } = useMarketStore()
  const [chartData, setChartData] = useState<{time: number, value: number}[]>([])
  const [chartTab, setChartTab] = useState('1D')
  
  // Generate realistic mock history
  useEffect(() => {
    if (polyMarketHistory.length > 0) {
      setChartData(polyMarketHistory)
    } else {
      const now = Math.floor(Date.now() / 1000)
      const data = []
      let lastVal = market.yes_price
      const points = 48
      const step = 3600
      
      for (let i = 0; i < points; i++) {
        const time = now - (points - i) * step
        const change = (Math.random() - 0.48) * 0.02
        lastVal = Math.max(0.01, Math.min(0.99, lastVal + change))
        data.push({ time, value: lastVal })
      }
      setChartData(data)
    }
  }, [market, polyMarketHistory])

  const prob = side === 'YES' ? market.yes_price : market.no_price
  const shares = amount ? (parseFloat(amount) / prob).toFixed(1) : "—"
  const potentialReturn = amount ? ((parseFloat(amount) / prob) - parseFloat(amount)).toFixed(2) : "—"

  return (
    <motion.div 
      initial={{ opacity: 0, x: 20 }}
      animate={{ opacity: 1, x: 0 }}
      className="flex flex-col h-full bg-[var(--bg1)] border-l border-[var(--bd)] w-[450px] shrink-0 overflow-hidden font-mono"
    >
      {/* SECTION HEADER */}
      <div className="h-[44px] border-b border-[var(--bd)] flex items-center justify-between px-4 bg-[var(--bg1)] shrink-0">
        <span className="text-[10px] font-bold text-[var(--tx3)] uppercase tracking-widest">Market Detail</span>
        <button onClick={onBack} className="p-1 hover:bg-[var(--bd)] rounded transition-colors text-[var(--tx3)] hover:text-white">
          <X className="w-4 h-4" />
        </button>
      </div>

      <div className="flex-1 overflow-y-auto no-scrollbar">
        {/* MARKET TITLE AREA */}
        <div className="p-6 flex flex-col gap-4 border-b border-[var(--bd)] bg-[var(--bg)]">
           <h2 className="text-[16px] font-bold text-white uppercase leading-tight tracking-tight">
             {market.question}
           </h2>
           <div className="flex items-center gap-4">
              <span className="px-2 py-0.5 bg-[var(--bd2)] text-[var(--tx2)] text-[9px] font-bold rounded-[2px] uppercase">{market.category}</span>
              <div className="flex gap-4">
                 <div>
                    <div className="text-[9px] text-[var(--tx3)] uppercase tracking-tighter">&gt;YES</div>
                    <div className="text-[20px] font-bold text-[var(--green)]">{(market.yes_price * 100).toFixed(0)}¢</div>
                 </div>
                 <div>
                    <div className="text-[9px] text-[var(--tx3)] uppercase tracking-tighter">&gt;NO</div>
                    <div className="text-[20px] font-bold text-[var(--red)]">{(market.no_price * 100).toFixed(0)}¢</div>
                 </div>
              </div>
           </div>
        </div>

        {/* CHART AREA */}
        <div className="flex flex-col border-b border-[var(--bd)]">
           <div className="p-4 flex items-center gap-2">
              {['1H', '6H', '1D', '1W', 'ALL'].map(t => (
                <button 
                  key={t}
                  onClick={() => setChartTab(t)}
                  className={`px-3 py-1 text-[10px] font-bold uppercase rounded-none border transition-all ${
                    chartTab === t ? 'text-[var(--amber)] border-[var(--bd)] bg-[var(--bg2)]' : 'text-[var(--tx3)] border-transparent hover:text-[var(--tx2)]'
                  }`}
                >
                  {t}
                </button>
              ))}
           </div>
           <div className="h-[200px] w-full bg-[var(--bg)]">
              <PolyChart data={chartData} color="var(--green)" />
           </div>
        </div>

        {/* MARKET INFO */}
        <div className="p-4 flex flex-col gap-2 border-b border-[var(--bd)]">
           <h3 className="text-[10px] font-bold text-[var(--tx3)] uppercase tracking-widest mb-2">Market Info</h3>
           <InfoRow label="Volume" value={formatVolume(market.volume_24h)} />
           <InfoRow label="Liquidity" value={formatUsd(market.liquidity)} />
           <InfoRow label="Created" value="2D AGO" />
           <InfoRow label="Resolves" value={new Date(market.end_date).toLocaleDateString(undefined, { month: 'short', day: 'numeric' }).toUpperCase()} />
        </div>

        {/* BET PANEL */}
        <div className="flex flex-col">
           <div className="flex border-b border-[var(--bd)]">
              <button 
                onClick={() => setSide('YES')}
                className={`flex-1 py-3 text-[11px] font-bold tracking-widest transition-all border-b-2 ${
                  side === 'YES' ? 'text-[var(--green)] border-[var(--green)]' : 'text-[var(--tx3)] border-transparent'
                }`}
              >
                BUY YES
              </button>
              <button 
                onClick={() => setSide('NO')}
                className={`flex-1 py-3 text-[11px] font-bold tracking-widest transition-all border-b-2 ${
                  side === 'NO' ? 'text-[var(--red)] border-[var(--red)]' : 'text-[var(--tx3)] border-transparent'
                }`}
              >
                BUY NO
              </button>
           </div>
           
           <div className="p-4 flex flex-col gap-4">
              <div className="flex items-center justify-between">
                 <span className="text-[10px] text-[var(--tx3)] uppercase tracking-widest font-bold">Amount (USDC)</span>
                 <input 
                   type="number"
                   value={amount}
                   onChange={e => setAmount(e.target.value)}
                   placeholder="0.00"
                   className="bg-transparent border-none text-right outline-none text-white font-mono text-sm w-1/2"
                 />
              </div>
              <div className="grid grid-cols-4 gap-2">
                 {['10', '25', '50', '100'].map(a => (
                   <button key={a} onClick={() => setAmount(a)} className="py-1.5 border border-[var(--bd)] text-[10px] text-[var(--tx2)] hover:border-[var(--bd2)] transition-all font-bold">
                     ${a}
                   </button>
                 ))}
              </div>
              
              <div className="flex flex-col gap-2 mt-2">
                 <DetailRow label="Avg Price" value={`${(prob * 100).toFixed(0)}¢`} />
                 <DetailRow label="Shares" value={shares} />
                 <DetailRow label="Max Return" value={potentialReturn !== '—' ? `+$${potentialReturn}` : '—'} green />
              </div>

              <button className={`w-full py-4 mt-2 text-[11px] font-bold uppercase tracking-[2px] transition-all active:scale-95 ${
                side === 'YES' ? 'bg-[var(--green)] text-black hover:bg-[#00c853]' : 'bg-[var(--red)] text-white hover:bg-[#cc3333]'
              }`}>
                Buy {side} @ {(prob * 100).toFixed(0)}¢
              </button>
           </div>
        </div>

        {/* ORDER BOOK (Simplified mock for UI) */}
        <div className="h-[44px] border-y border-[var(--bd)] flex items-center px-4 bg-[var(--bg1)] shrink-0">
          <span className="text-[10px] font-bold text-[var(--tx3)] uppercase tracking-widest">Order Book</span>
        </div>
        <div className="p-4 overflow-hidden">
           <table className="w-full text-[10px] font-mono">
              <thead>
                 <tr className="text-[var(--tx3)] text-left uppercase">
                    <th className="pb-2 font-bold tracking-widest">Price</th>
                    <th className="pb-2 font-bold tracking-widest">Shares</th>
                    <th className="pb-2 font-bold tracking-widest text-right">Value</th>
                 </tr>
              </thead>
              <tbody>
                 {[0,1,2].map(i => (
                   <tr key={`ask-${i}`} className="relative h-6">
                      <td className="text-[var(--red)] font-bold">{(market.yes_price + 0.01 + i*0.01).toFixed(2)}</td>
                      <td>{(Math.random() * 5000 + 500).toFixed(0)}</td>
                      <td className="text-right text-[var(--tx2)]">${(Math.random() * 1000).toFixed(0)}</td>
                   </tr>
                 ))}
                 <tr className="h-8 border-y border-[var(--bd)]/50">
                    <td colSpan={3} className="text-center text-[var(--amber)] font-bold tracking-widest">MID {(market.yes_price).toFixed(2)}</td>
                 </tr>
                 {[0,1,2].map(i => (
                   <tr key={`bid-${i}`} className="relative h-6">
                      <td className="text-[var(--green)] font-bold">{(market.yes_price - 0.01 - i*0.01).toFixed(2)}</td>
                      <td>{(Math.random() * 5000 + 500).toFixed(0)}</td>
                      <td className="text-right text-[var(--tx2)]">${(Math.random() * 1000).toFixed(0)}</td>
                   </tr>
                 ))}
              </tbody>
           </table>
        </div>
      </div>
    </motion.div>
  )
}

function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between items-center text-[11px]">
       <span className="text-[var(--tx3)] uppercase tracking-tighter">{label}</span>
       <span className="text-[var(--tx)] font-bold">{value}</span>
    </div>
  )
}

function DetailRow({ label, value, green }: { label: string; value: string; green?: boolean }) {
  return (
    <div className="flex justify-between items-center text-[11px]">
       <span className="text-[var(--tx3)] uppercase tracking-tighter">{label}</span>
       <span className={`font-bold ${green ? 'text-[var(--green)]' : 'text-[var(--tx)]'}`}>{value}</span>
    </div>
  )
}
