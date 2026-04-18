'use client'
import React, { useState, useMemo } from 'react'
import { useMarketStore, PolyOpportunity } from '@/hooks/useMarketStore'
import { useEngineWS } from '@/hooks/useEngineWS'
import { Search, RefreshCw, LayoutGrid, ListFilter, TrendingUp, Globe, Cpu, Flame, Trophy } from 'lucide-react'
import { PolyMarketCard } from './PolyMarketCard'
import { motion, AnimatePresence } from 'framer-motion'

const CATEGORIES = [
  { id: 'all', label: 'All Markets', icon: LayoutGrid },
  { id: 'crypto', label: 'Crypto', icon: Cpu },
  { id: 'politics', label: 'Politics', icon: Globe },
  { id: 'sports', label: 'Sports', icon: Trophy },
  { id: 'trending', label: 'Trending', icon: Flame },
]

interface Props {
  initialCategory?: string
  hideHeader?: boolean
  searchOverride?: string
}

export function PolymarketPanel({ initialCategory = 'all', hideHeader = false, searchOverride = '' }: Props) {
  const { polyOpportunities, selectedPolyMarketId, setSelectedPolyMarket } = useMarketStore()
  const { sendMessage } = useEngineWS()
  const [internalSearch, setInternalSearch] = useState('')
  const [activeCategory, setActiveCategory] = useState(initialCategory)

  const search = searchOverride || internalSearch

  React.useEffect(() => {
    setActiveCategory(initialCategory)
  }, [initialCategory])

  const filtered = useMemo(() => {
    return polyOpportunities.filter(o => {
      const matchesSearch = o.question.toLowerCase().includes(search.toLowerCase()) ||
                          o.category.toLowerCase().includes(search.toLowerCase())
      const matchesCategory = activeCategory === 'all' || 
                             o.category.toLowerCase() === activeCategory ||
                             (activeCategory === 'trending' && o.volume_24h > 100000)
      return matchesSearch && matchesCategory
    })
  }, [polyOpportunities, search, activeCategory])

  return (
    <div className="flex flex-col h-full bg-[#050608]">
      {/* SEARCH & FILTERS HEADER */}
      {!hideHeader && (
        <div className="p-4 border-b border-[#1e2634] bg-[#0d1117]/50 backdrop-blur-sm shrink-0 flex flex-col gap-4">
          <div className="flex items-center gap-3">
            <div className="relative flex-1 group">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-[#4a5568] group-focus-within:text-blue-500 transition-colors" />
              <input 
                value={internalSearch} 
                onChange={e => setInternalSearch(e.target.value)} 
                placeholder="Search prediction markets…"
                className="w-full bg-[#111827] border border-[#1e2634] rounded-xl pl-10 pr-4 py-2 text-[#e2e8f0] outline-none group-hover:border-[#3b82f6]/40 focus:border-[#3b82f6] transition-all text-xs" 
              />
            </div>
            <button 
              onClick={() => sendMessage({ type: 'get_poly_opportunities' })}
              className="p-2.5 bg-[#111827] border border-[#1e2634] rounded-xl text-[#4a5568] hover:text-[#3b82f6] hover:border-[#3b82f6]/40 transition-all active:scale-95">
              <RefreshCw className="w-4 h-4" />
            </button>
          </div>

          <div className="flex items-center gap-2 overflow-x-auto no-scrollbar pb-1">
            {CATEGORIES.map(cat => (
              <button
                key={cat.id}
                onClick={() => setActiveCategory(cat.id)}
                className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-[10px] font-bold uppercase tracking-wider whitespace-nowrap border transition-all ${
                  activeCategory === cat.id 
                    ? 'bg-blue-500/10 border-blue-500/30 text-blue-400' 
                    : 'bg-[#111827] border-[#1e2634] text-[#4a5568] hover:border-[#3b82f6]/40 hover:text-[#94a3b8]'
                }`}
              >
                <cat.icon className="w-3.5 h-3.5" />
                {cat.label}
              </button>
            ))}
          </div>
        </div>
      )}

      {/* MARKET LIST */}
      <div className="flex-1 overflow-auto no-scrollbar">
        {filtered.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-64 gap-4 opacity-40">
            <RefreshCw className="w-8 h-8 animate-spin text-[#4a5568]" />
            <div className="text-center">
              <p className="text-xs font-bold uppercase tracking-widest text-[#4a5568]">Syncing Markets</p>
            </div>
          </div>
        ) : (
          <div className="flex flex-col min-w-0">
            <AnimatePresence mode="popLayout">
              {filtered.map(opp => (
                <PolyMarketCard 
                  key={opp.market_id} 
                  market={opp} 
                  active={selectedPolyMarketId === opp.market_id}
                  onClick={() => setSelectedPolyMarket(opp.market_id)}
                />
              ))}
            </AnimatePresence>
          </div>
        )}
      </div>

      {/* FOOTER STATS */}
      <div className="p-3 bg-[#0d1117] border-t border-[#1e2634] flex items-center justify-between">
        <div className="flex items-center gap-4">
          <div className="flex flex-col">
            <span className="text-[9px] font-bold text-[#4a5568] uppercase leading-none">Total Liquidity</span>
            <span className="text-[11px] font-mono font-bold text-[#e2e8f0]">$2.4M</span>
          </div>
        </div>
        <div className="flex items-center gap-2 px-2 py-1 rounded-full bg-emerald-500/10 border border-emerald-500/20">
          <div className="w-1 h-1 rounded-full bg-emerald-500 animate-pulse" />
          <span className="text-[9px] font-bold text-emerald-500 uppercase">Live</span>
        </div>
      </div>
    </div>
  )
}
