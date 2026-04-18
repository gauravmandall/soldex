'use client'
import React, { useState, useEffect } from 'react'
import { TopBar } from '@/components/terminal/TopBar'
import { useEngineWS } from '@/hooks/useEngineWS'
import { useMarketStore } from '@/hooks/useMarketStore'
import { PolymarketPanel } from '@/components/polymarket/PolymarketPanel'
import { PolyMarketDetail } from '@/components/polymarket/PolyMarketDetail'
import { 
  LayoutGrid, Globe, Cpu, Trophy, Flame, 
  ChevronRight, Terminal, ChevronDown, 
  Search as SearchIcon, Activity
} from 'lucide-react'
import { AnimatePresence } from 'framer-motion'

const CATEGORIES = [
  { id: 'all', label: 'All Markets', icon: LayoutGrid },
  { id: 'crypto', label: 'Crypto', icon: Cpu },
  { id: 'politics', label: 'Politics', icon: Globe },
  { id: 'sports', label: 'Sports', icon: Trophy },
  { id: 'trending', label: 'Trending', icon: Flame },
]

const TICKERS = [
  { name: "BTC/YES", val: "67¢", chg: "+2.1%", up: true },
  { name: "ETH/YES", val: "42¢", chg: "-1.3%", up: false },
  { name: "SOL/YES", val: "18¢", chg: "+0.8%", up: true },
  { name: "XRP/YES", val: "71¢", chg: "+3.2%", up: true },
  { name: "FED/YES", val: "55¢", chg: "-0.5%", up: false },
  { name: "TRUMP/YES", val: "62¢", chg: "+1.1%", up: true },
]

const GLOBAL_METRICS = [
  { l: "24H VOL", v: "$89.4M", cls: "up" },
  { l: "MARKETS", v: "2,841", cls: "" },
  { l: "LIQUIDITY", v: "$14.2M", cls: "up" },
  { l: "GAS", v: "0.001 MATIC", cls: "" },
  { l: "BLOCK", v: "#58,221,401", cls: "" },
  { l: "TRADERS", v: "12,488", cls: "up" },
]

export default function PolymarketPage() {
  const { connected } = useEngineWS()
  const { polyOpportunities, selectedPolyMarketId, setSelectedPolyMarket } = useMarketStore()
  const [activeCategory, setActiveCategory] = useState('all')
  const [search, setSearch] = useState('')
  const [time, setTime] = useState('')

  const selectedMarket = polyOpportunities.find(o => o.market_id === selectedPolyMarketId)

  useEffect(() => {
    const t = setInterval(() => setTime(new Date().toISOString().slice(11, 19)), 1000)
    return () => clearInterval(t)
  }, [])

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === '/' && document.activeElement?.tagName !== 'INPUT') {
        e.preventDefault()
        document.getElementById('sidebar-search')?.focus()
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [])

  return (
    <div className="flex flex-col h-screen bg-[var(--bg)] text-[var(--tx)] font-mono selection:bg-blue-500/30 overflow-hidden">
      {/* UNIVERSAL TOP BAR */}
      <TopBar connected={connected} />

      {/* METRICS BAR */}
      <div className="h-[36px] border-b border-[var(--bd)] flex items-center px-4 gap-8 bg-[var(--bg1)] shrink-0 overflow-hidden">
        {GLOBAL_METRICS.map((m, i) => (
          <React.Fragment key={m.l}>
            <div className="flex items-center gap-2 whitespace-nowrap">
              <span className="text-[10px] text-[var(--tx3)] tracking-widest uppercase">{m.l}</span>
              <span className={`text-[11px] font-bold ${m.cls === 'up' ? 'text-[var(--green)]' : 'text-[var(--tx)]'}`}>{m.v}</span>
            </div>
            {i < GLOBAL_METRICS.length - 1 && <div className="w-[1px] h-4 bg-[var(--bd)]" />}
          </React.Fragment>
        ))}
        <div className="ml-auto flex items-center gap-2">
           <div className="w-1.5 h-1.5 rounded-full bg-[var(--green)] animate-pulse shadow-[0_0_8px_var(--green)]" />
           <span className="text-[10px] text-[var(--tx2)]">UTC <span className="text-[var(--tx)]">{time}</span></span>
        </div>
      </div>
      
      <div className="flex flex-1 min-h-0">
        {/* SIDEBAR (LEFT) */}
        <aside className="w-[280px] border-r border-[var(--bd)] bg-[var(--bg1)] flex flex-col shrink-0">
          <div className="p-5 flex flex-col gap-6 flex-1 overflow-y-auto no-scrollbar">
            {/* SEARCH */}
            <div className="relative group">
              <SearchIcon className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-[var(--tx3)] group-focus-within:text-[var(--blue)] transition-colors" />
              <input 
                id="sidebar-search"
                value={search}
                onChange={e => setSearch(e.target.value)}
                placeholder="SEARCH MARKETS..."
                className="w-full bg-[var(--bg2)] border border-[var(--bd)] rounded-none pl-9 pr-3 py-2 text-xs text-[var(--tx)] outline-none group-hover:border-[var(--bd2)] focus:border-[var(--blue)] transition-all uppercase placeholder:text-[var(--tx3)]"
              />
              <div className="absolute right-3 top-1/2 -translate-y-1/2 px-1.5 py-0.5 bg-[var(--bd)] border border-[var(--bd2)] text-[9px] font-bold text-[var(--tx3)]">/</div>
            </div>

            {/* CATEGORIES */}
            <div className="flex flex-col gap-2">
              <h3 className="text-[10px] font-bold text-[var(--tx3)] uppercase tracking-widest px-3 flex items-center justify-between mb-2">
                Categories
                <ChevronDown className="w-3 h-3" />
              </h3>
              <nav className="flex flex-col gap-0.5">
                {CATEGORIES.map(cat => (
                  <button
                    key={cat.id}
                    onClick={() => setActiveCategory(cat.id)}
                    className={`flex items-center gap-3 px-3 py-2 text-xs font-bold transition-all group ${
                      activeCategory === cat.id 
                        ? 'text-[var(--amber)] bg-[var(--bg2)] border border-[var(--bd2)]' 
                        : 'text-[var(--tx2)] hover:text-[var(--tx)] hover:bg-[var(--bg2)]'
                    }`}
                  >
                    <cat.icon className={`w-4 h-4 ${activeCategory === cat.id ? 'text-[var(--amber)]' : 'text-[var(--tx3)] group-hover:text-[var(--tx2)]'}`} />
                    {cat.label.toUpperCase()}
                  </button>
                ))}
              </nav>
            </div>
          </div>
        </aside>

        {/* MARKET LIST (CENTER) */}
        <main className="flex-1 flex flex-col bg-[var(--bg)] relative min-w-0">
          <div className="h-[44px] border-b border-[var(--bd)] bg-[var(--bg2)] flex items-center px-6 gap-4 shrink-0">
             <div className="flex items-center gap-2 text-[10px] font-bold text-[var(--tx3)] uppercase tracking-widest">
                <span>Markets</span>
                <ChevronRight className="w-3 h-3" />
                <span className="text-[var(--tx)]">{activeCategory}</span>
             </div>
             <div className="ml-auto flex items-center gap-4">
                <div className="flex items-center gap-4 px-3 py-1">
                   <span className="text-[10px] text-[var(--tx3)] uppercase">Sort by:</span>
                   {['Volume', 'Liquidity', 'Newest'].map(s => (
                     <button key={s} className="text-[10px] text-[var(--tx2)] hover:text-[var(--blue)] uppercase font-bold">{s}</button>
                   ))}
                </div>
             </div>
          </div>

          <div className="flex-1 overflow-y-auto no-scrollbar">
            <PolymarketPanel 
              initialCategory={activeCategory} 
              hideHeader 
              searchOverride={search}
            />
          </div>
        </main>

        {/* MARKET DETAIL (RIGHT PANEL) */}
        <AnimatePresence mode="wait">
          {selectedMarket && (
            <PolyMarketDetail 
              key={selectedMarket.market_id}
              market={selectedMarket} 
              onBack={() => setSelectedPolyMarket(null)} 
            />
          )}
        </AnimatePresence>
      </div>

      {/* TICKER STRIP */}
      <div className="h-[32px] border-t border-[var(--bd)] bg-[var(--bg1)] flex items-center overflow-hidden whitespace-nowrap px-4 shrink-0">
         <div className="flex items-center gap-12 ticker-scroll">
            {[...TICKERS, ...TICKERS, ...TICKERS].map((t, i) => (
              <div key={i} className="flex items-center gap-2 text-[10px] font-bold">
                 <span className="text-[var(--tx2)] tracking-tighter">{t.name}</span>
                 <span className="text-[var(--green)] font-mono">{t.val}</span>
                 <span className={`${t.up ? 'text-[var(--green)]' : 'text-[var(--red)]'} font-mono`}>{t.chg}</span>
              </div>
            ))}
         </div>
      </div>
    </div>
  )
}
