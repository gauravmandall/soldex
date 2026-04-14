'use client'
import React from 'react'
import { TopBar } from '@/components/terminal/TopBar'
import { useEngineWS } from '@/hooks/useEngineWS'
import { PolymarketPanel } from '@/components/polymarket/PolymarketPanel'

export default function PolymarketPage() {
  const { connected } = useEngineWS()
  return (
    <div className="flex flex-col h-screen bg-[#0a0b0f]">
      <TopBar connected={connected} />
      <div className="flex-1 flex flex-col overflow-hidden p-6 max-w-4xl mx-auto w-full">
        <h1 className="text-xl font-bold text-white mb-4">Polymarket Discovery</h1>
        <p className="text-xs text-[#4b5068] mb-6">Browse and execute Polymarket prediction market orders directly from Soldex with self-custody privacy.</p>
        <div className="flex-1 bg-[#13151d] border border-[#1e2130] rounded overflow-hidden">
          <PolymarketPanel />
        </div>
      </div>
    </div>
  )
}
