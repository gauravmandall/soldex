'use client'
import React, { useState, useEffect } from 'react'
import { TopBar } from '@/components/terminal/TopBar'
import { useEngineWS } from '@/hooks/useEngineWS'
import { useMarketStore, PolyOpportunity, JupiterOpportunity } from '@/hooks/useMarketStore'
import { PolymarketPanel } from '@/components/polymarket/PolymarketPanel'
import { PolyMarketDetail } from '@/components/polymarket/PolyMarketDetail'
import { PendingTxModal } from '@/components/terminal/PendingTxModal'
import { 
  LayoutGrid, Globe, Cpu, Trophy, Flame, 
  ChevronRight, Terminal, ChevronDown, 
  Search as SearchIcon, Activity, Circle, Gamepad2, Music, TrendingUp, Cloud, AtSign
} from 'lucide-react'
import { AnimatePresence } from 'framer-motion'

const CATEGORIES = [
  { id: 'all', label: 'All' },
  { id: 'live', label: 'Live', dot: true },
  { id: 'featured', label: '', icon: Trophy },
  { id: 'separator', type: 'separator' },
  { id: 'sports', label: 'Sports' },
  { id: 'crypto', label: 'Crypto' },
  { id: 'politics', label: 'Politics' },
  { id: 'esports', label: 'Esports' },
  { id: 'culture', label: 'Culture' },
  { id: 'economics', label: 'Economics' },
  { id: 'tech', label: 'Tech' },
  { id: 'finance', label: 'Finance' },
  { id: 'weather', label: 'Weather' },
  { id: 'mentions', label: 'Mentions' },
]

const TICKERS = [
  { name: "BTC/YES", val: "67¢", chg: "+2.1%", up: true },
  { name: "ETH/YES", val: "42¢", chg: "-1.3%", up: false },
  { name: "SOL/YES", val: "18¢", chg: "+0.8%", up: true },
  { name: "XRP/YES", val: "71¢", chg: "+3.2%", up: true },
  { name: "FED/YES", val: "55¢", chg: "-0.5%", up: false },
  { name: "TRUMP/YES", val: "62¢", chg: "+1.1%", up: true },
]

export default function DiscoveryPage() {
  const { connected, sendMessage } = useEngineWS()
  const { 
    polyOpportunities, 
    jupiterOpportunities, 
    selectedPolyMarketId, 
    setSelectedPolyMarket,
    selectedJupiterMarketId,
    setSelectedJupiterMarket
  } = useMarketStore()
  const [activeCategory, setActiveCategory] = useState('all')
  const [search, setSearch] = useState('')
  const [time, setTime] = useState('')

  const selectedMarket = (polyOpportunities.find(o => o.market_id === selectedPolyMarketId) ||
                         jupiterOpportunities.find(o => o.market_id === selectedJupiterMarketId)) as PolyOpportunity | JupiterOpportunity | undefined

  const clearSelection = () => {
    setSelectedPolyMarket(null)
    setSelectedJupiterMarket(null)
  }

  useEffect(() => {
    const t = setInterval(() => setTime(new Date().toISOString().slice(11, 19)), 1000)
    return () => clearInterval(t)
  }, [])

  useEffect(() => {
    if (connected) {
      sendMessage({ type: 'get_poly_opportunities' })
      sendMessage({ type: 'get_jupiter_opportunities' })
    }
  }, [connected])

  return (
    <div className="flex flex-col h-screen bg-[#060709] text-[var(--tx)] font-sans selection:bg-blue-500/30 overflow-hidden">
      {/* UNIVERSAL TOP BAR */}
      <TopBar connected={connected} />

      {/* SUB-HEADER / CATEGORY NAV */}
      <div className="h-[48px] border-b border-[#1e2634] flex items-center px-6 gap-2 bg-[#0d1117] shrink-0 overflow-x-auto no-scrollbar">
        {CATEGORIES.map((cat, idx) => {
          if (cat.type === 'separator') {
            return <div key={idx} className="h-4 w-[1px] bg-[#1e2634] mx-2" />
          }

          const Icon = cat.icon
          return (
            <button
              key={cat.id}
              onClick={() => setActiveCategory(cat.id)}
              className={`flex items-center gap-1.5 px-3 py-4 text-[12px] font-medium transition-all whitespace-nowrap border-b-2 h-full ${
                activeCategory === cat.id 
                  ? 'text-white border-white' 
                  : 'text-[#4a5568] border-transparent hover:text-white'
              }`}
            >
              {cat.dot && <div className="w-1.5 h-1.5 rounded-full bg-rose-500 animate-pulse" />}
              {Icon && <Icon className={`w-3.5 h-3.5 ${cat.id === 'featured' ? 'text-yellow-500' : (activeCategory === cat.id ? 'text-white' : 'text-[#4a5568]')}`} />}
              {cat.label}
            </button>
          )
        })}
        <div className="ml-auto flex items-center gap-4">
           <div className="relative group w-[240px]">
              <SearchIcon className="absolute left-3 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-[#4a5568] group-focus-within:text-blue-500 transition-colors" />
              <input 
                value={search}
                onChange={e => setSearch(e.target.value)}
                placeholder="SEARCH MARKETS..."
                className="w-full bg-[#111827] border border-[#1e2634] rounded-full pl-9 pr-3 py-1.5 text-[10px] text-white outline-none group-hover:border-[#3b82f6]/40 focus:border-[#3b82f6] transition-all uppercase placeholder:text-[#4a5568]"
              />
           </div>
        </div>
      </div>
      
      <div className="flex-1 overflow-y-auto no-scrollbar pb-12">
        <div className="max-w-[1400px] mx-auto px-6 py-8">
           <AnimatePresence mode="wait">
             {selectedMarket ? (
               <div className="flex gap-8 items-start">
                  <div className="flex-1 min-w-0">
                    <PolyMarketDetail 
                      key={selectedMarket.market_id}
                      market={selectedMarket} 
                      onBack={clearSelection} 
                    />
                  </div>
               </div>
             ) : (
               <div className="flex flex-col gap-12">
                 <div className="flex flex-col gap-6">
                    <div className="flex items-center justify-between border-b border-[#1e2634] pb-4">
                       <h2 className="text-xl font-bold tracking-tight text-white flex items-center gap-3">
                          {activeCategory === 'all' ? 'Market Discovery' : (CATEGORIES.find(c => c.id === activeCategory)?.label || 'Featured Markets')}
                          <span className="px-2 py-0.5 bg-[#1e2634] text-[10px] text-[#4a5568] rounded-md font-mono">
                             {(polyOpportunities.length + jupiterOpportunities.length)} ACTIVE
                          </span>
                       </h2>
                       <div className="flex items-center gap-2">
                          <button className="p-2 text-[#4a5568] hover:text-white transition-colors"><LayoutGrid className="w-4 h-4" /></button>
                          <button className="p-2 text-[#4a5568] hover:text-white transition-colors"><Terminal className="w-4 h-4" /></button>
                       </div>
                    </div>

                    <PolymarketPanel 
                      initialCategory={activeCategory} 
                      hideHeader 
                      searchOverride={search}
                    />
                 </div>
               </div>
             )}
           </AnimatePresence>
        </div>
      </div>

      <PendingTxModal />

      {/* TICKER STRIP */}
      <div className="h-[32px] border-t border-[#1e2634] bg-[#0d1117] flex items-center overflow-hidden whitespace-nowrap px-4 shrink-0">
         <div className="flex items-center gap-12 ticker-scroll">
            {[...TICKERS, ...TICKERS, ...TICKERS].map((t, i) => (
              <div key={i} className="flex items-center gap-2 text-[9px] font-bold">
                 <span className="text-[#4a5568] tracking-tighter uppercase">{t.name}</span>
                 <span className="text-emerald-500 font-mono">{t.val}</span>
                 <span className={`${t.up ? 'text-emerald-500' : 'text-rose-500'} font-mono`}>{t.chg}</span>
              </div>
            ))}
         </div>
      </div>
    </div>
  )
}
