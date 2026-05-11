'use client'
import React, { useState, useEffect } from 'react'
import { TopBar } from '@/components/terminal/TopBar'
import { useEngineWS } from '@/hooks/useEngineWS'
import { useMarketStore } from '@/hooks/useMarketStore'
import { PendingTxModal } from '@/components/terminal/PendingTxModal'

const TICKERS = [
  { name: "BTC/YES", val: "67¢", chg: "+2.1%", up: true },
  { name: "ETH/YES", val: "42¢", chg: "-1.3%", up: false },
  { name: "SOL/YES", val: "18¢", chg: "+0.8%", up: true },
  { name: "XRP/YES", val: "71¢", chg: "+3.2%", up: true },
  { name: "FED/YES", val: "55¢", chg: "-0.5%", up: false },
  { name: "TRUMP/YES", val: "62¢", chg: "+1.1%", up: true },
]

export default function PredictionLayout({ children }: { children: React.ReactNode }) {
  const { connected, sendMessage } = useEngineWS()
  const { polyOpportunities, jupiterOpportunities } = useMarketStore()

  useEffect(() => {
    if (connected) {
      sendMessage({ type: 'get_poly_opportunities' })
      sendMessage({ type: 'get_jupiter_opportunities' })
    }
  }, [connected, sendMessage])

  return (
    <div className="flex flex-col h-screen bg-[#060709] text-[var(--tx)] font-sans selection:bg-blue-500/30 overflow-hidden">
      {/* UNIVERSAL TOP BAR */}
      <TopBar connected={connected} />

      <div className="flex-1 overflow-y-auto no-scrollbar pb-12">
        {children}
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
