'use client'
import React, { useState } from 'react'
import { useMarketStore } from '@/hooks/useMarketStore'
import { useEngineWS } from '@/hooks/useEngineWS'
import { TrendingUp, RefreshCw } from 'lucide-react'

export function PolymarketPanel() {
  const { polyOpportunities } = useMarketStore()
  const { sendMessage } = useEngineWS()
  const [search, setSearch] = useState('')

  const filtered = polyOpportunities.filter(o =>
    o.question.toLowerCase().includes(search.toLowerCase()) ||
    o.category.toLowerCase().includes(search.toLowerCase())
  )

  return (
    <div className="flex flex-col h-full text-xs font-mono bg-[#0a0e15]">
      <div className="p-3 border-b border-[#1e2634] flex gap-2 shrink-0 bg-[#0d1117]">
        <div className="relative flex-1 group">
          <input 
            value={search} 
            onChange={e => setSearch(e.target.value)} 
            placeholder="Search prediction markets…"
            className="w-full bg-[#111827] border border-[#1e2634] rounded-md px-3 py-1.5 text-[#e2e8f0] outline-none group-hover:border-[#3b82f6]/40 focus:border-[#3b82f6] transition-all text-[11px]" 
          />
        </div>
        <button 
          onClick={() => sendMessage({ type: 'get_poly_opportunities' })}
          className="p-2 bg-[#111827] border border-[#1e2634] rounded-md text-[#4a5568] hover:text-[#3b82f6] hover:border-[#3b82f6]/40 transition-all active:scale-95">
          <RefreshCw className="w-3.5 h-3.5" />
        </button>
      </div>
      <div className="flex-1 overflow-auto no-scrollbar">
        {filtered.length === 0 && (
          <div className="flex flex-col items-center justify-center h-full gap-3 opacity-40">
            <RefreshCw className="w-6 h-6 animate-spin text-[#4a5568]" />
            <span className="text-[10px] font-bold uppercase tracking-widest text-[#4a5568]">Loading Markets…</span>
          </div>
        )}
        {filtered.map(opp => (
          <div key={opp.market_id} className="p-4 border-b border-[#1e2634] hover:bg-[#111827] cursor-pointer transition-all group">
            <div className="flex items-start justify-between gap-2 mb-2.5">
              <span className="text-[9px] font-bold text-[#3b82f6] bg-[#1e3a5f]/30 px-1.5 py-0.5 rounded tracking-wider uppercase">
                {opp.category}
              </span>
              <div className="flex items-center gap-1.5">
                <div className="w-1.5 h-1.5 rounded-full bg-[#10b981] shadow-[0_0_8px_rgba(16,185,129,0.4)]" />
                <span className="text-[10px] font-bold text-[#4a5568] whitespace-nowrap uppercase">
                  Vol ${(opp.volume_24h / 1000).toFixed(0)}K
                </span>
              </div>
            </div>
            <p className="text-[12px] font-bold text-[#e2e8f0] leading-snug mb-3 line-clamp-2 group-hover:text-[#3b82f6] transition-colors">
              {opp.question}
            </p>
            <div className="flex items-center gap-3">
              <div className="flex-1">
                <div className="flex justify-between text-[10px] font-bold mb-1.5">
                  <span className="text-[#10b981]">YES</span>
                  <span className="text-[#e2e8f0] num">{(opp.yes_price * 100).toFixed(1)}¢</span>
                </div>
                <div className="h-1.5 bg-[#1e2634] rounded-full overflow-hidden">
                  <div className="h-full bg-gradient-to-r from-[#059669] to-[#10b981] rounded-full shadow-[0_0_12px_rgba(16,185,129,0.2)]" style={{ width: `${opp.yes_price * 100}%` }} />
                </div>
              </div>
              <div className="flex-1">
                <div className="flex justify-between text-[10px] font-bold mb-1.5">
                  <span className="text-[#ef4444]">NO</span>
                  <span className="text-[#e2e8f0] num">{(opp.no_price * 100).toFixed(1)}¢</span>
                </div>
                <div className="h-1.5 bg-[#1e2634] rounded-full overflow-hidden">
                  <div className="h-full bg-gradient-to-r from-[#be123c] to-[#ef4444] rounded-full shadow-[0_0_12px_rgba(239,68,68,0.2)]" style={{ width: `${opp.no_price * 100}%` }} />
                </div>
              </div>
            </div>
            <div className="flex justify-between mt-3 text-[9px] font-bold text-[#4a5568] uppercase tracking-tighter">
              <span className="flex items-center gap-1"><span className="text-[#94a3b8]">Spread</span> {(opp.spread * 100).toFixed(2)}¢</span>
              <span className="flex items-center gap-1"><span className="text-[#94a3b8]">Liq</span> ${(opp.liquidity / 1000).toFixed(0)}K</span>
              <span className="flex items-center gap-1"><span className="text-[#94a3b8]">Ends</span> {new Date(opp.end_date).toLocaleDateString()}</span>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
