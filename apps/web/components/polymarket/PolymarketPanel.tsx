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
    <div className="flex flex-col h-full text-xs font-mono">
      <div className="p-2 border-b border-[#1e2130] flex gap-2 shrink-0">
        <input value={search} onChange={e => setSearch(e.target.value)} placeholder="Search markets…"
          className="flex-1 bg-[#0a0b0f] border border-[#1e2130] rounded px-2 py-1 text-[#e2e4ef] outline-none focus:border-blue-500/60 text-[11px]" />
        <button onClick={() => sendMessage({ type: 'get_poly_opportunities' })}
          className="text-[#4b5068] hover:text-white transition-colors">
          <RefreshCw className="w-3.5 h-3.5" />
        </button>
      </div>
      <div className="flex-1 overflow-auto">
        {filtered.length === 0 && (
          <div className="flex items-center justify-center h-full text-[#4b5068]">Loading opportunities…</div>
        )}
        {filtered.map(opp => (
          <div key={opp.market_id} className="p-3 border-b border-[#1e2130] hover:bg-[#1a1d28] cursor-pointer transition-colors">
            <div className="flex items-start justify-between gap-2 mb-1.5">
              <span className="text-[10px] text-blue-400 uppercase">{opp.category}</span>
              <span className="text-[10px] text-[#4b5068] whitespace-nowrap">Vol ${(opp.volume_24h / 1000).toFixed(0)}K</span>
            </div>
            <p className="text-[11px] text-[#e2e4ef] leading-snug mb-2 line-clamp-2">{opp.question}</p>
            <div className="flex items-center gap-2">
              <div className="flex-1">
                <div className="flex justify-between text-[10px] mb-0.5">
                  <span className="text-green-400">YES</span>
                  <span className="text-green-400 num">{(opp.yes_price * 100).toFixed(1)}¢</span>
                </div>
                <div className="h-1.5 bg-[#13151d] rounded-full overflow-hidden">
                  <div className="h-full bg-green-500/60 rounded-full" style={{ width: `${opp.yes_price * 100}%` }} />
                </div>
              </div>
              <div className="flex-1">
                <div className="flex justify-between text-[10px] mb-0.5">
                  <span className="text-red-400">NO</span>
                  <span className="text-red-400 num">{(opp.no_price * 100).toFixed(1)}¢</span>
                </div>
                <div className="h-1.5 bg-[#13151d] rounded-full overflow-hidden">
                  <div className="h-full bg-red-500/60 rounded-full" style={{ width: `${opp.no_price * 100}%` }} />
                </div>
              </div>
            </div>
            <div className="flex justify-between mt-1.5 text-[10px] text-[#4b5068]">
              <span>Spread {(opp.spread * 100).toFixed(2)}¢</span>
              <span>Liq ${(opp.liquidity / 1000).toFixed(0)}K</span>
              <span>Ends {new Date(opp.end_date).toLocaleDateString()}</span>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
