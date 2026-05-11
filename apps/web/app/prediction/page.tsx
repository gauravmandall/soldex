'use client'
import React, { useState } from 'react'
import { useMarketStore } from '@/hooks/useMarketStore'
import { PolymarketPanel } from '@/components/polymarket/PolymarketPanel'
import { 
  LayoutGrid, Trophy, Terminal, 
  Search as SearchIcon
} from 'lucide-react'

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

export default function DiscoveryPage() {
  const { polyOpportunities, jupiterOpportunities } = useMarketStore()
  const [activeCategory, setActiveCategory] = useState('all')
  const [search, setSearch] = useState('')

  return (
    <>
      {/* SUB-HEADER / CATEGORY NAV */}
      <div className="h-[48px] border-b border-[#1e2634] flex items-center px-6 gap-2 bg-[#0d1117] shrink-0 overflow-x-auto no-scrollbar sticky top-0 z-10">
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
      
      <div className="max-w-[1400px] mx-auto px-6 py-8">
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
      </div>
    </>
  )
}
