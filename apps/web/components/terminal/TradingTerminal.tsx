'use client'

import React, { useState, useEffect } from 'react'
import { useMarketStore } from '@/hooks/useMarketStore'
import { useEngineWS } from '@/hooks/useEngineWS'
import { TopBar } from './TopBar'
import { MarketSelector } from './MarketSelector'
import { CandleChart } from '../charts/CandleChart'
import { OrderBook } from './OrderBook'
import { TradesPanel } from './TradesPanel'
import { OrderPanel } from './OrderPanel'
import { PositionsPanel } from './PositionsPanel'
import { PolymarketPanel } from '../polymarket/PolymarketPanel'
import { PrivacyBanner } from '../privacy/PrivacyBanner'

export type MarketId = 'SOL-PERP' | 'BTC-PERP' | 'ETH-PERP' | string
export type RightPanel = 'orderbook' | 'trades' | 'polymarket'
export type BottomPanel = 'positions' | 'orders' | 'history'

interface Props { defaultMarket?: string }

export function TradingTerminal({ defaultMarket = 'SOL-PERP' }: Props) {
  const { ticker, setActiveMarket, activeMarket } = useMarketStore()
  const { connected } = useEngineWS()
  const [rightPanel, setRightPanel] = useState<RightPanel>('orderbook')
  const [bottomPanel, setBottomPanel] = useState<BottomPanel>('positions')
  const [chartInterval, setChartInterval] = useState<'1m'|'5m'|'15m'|'1h'|'4h'|'1d'>('5m')

  useEffect(() => { setActiveMarket(defaultMarket) }, [defaultMarket, setActiveMarket])

  const pxColor = ticker
    ? (ticker.change_pct_24h >= 0 ? 'text-green-400' : 'text-red-400')
    : 'text-[#8b90a8]'

  return (
    <div className="flex flex-col h-screen w-screen bg-[#0a0b0f] overflow-hidden">
      <TopBar connected={connected} />

      {/* Ticker strip */}
      <div className="flex items-center gap-6 px-4 py-2 border-b border-[#1e2130] bg-[#0e1018] shrink-0">
        <MarketSelector />
        {ticker && (
          <>
            <div className="flex items-baseline gap-2">
              <span className={`text-2xl font-bold num ${pxColor}`}>
                {ticker.price.toLocaleString('en-US', { minimumFractionDigits: 2 })}
              </span>
              <span className={`text-xs num ${pxColor}`}>
                {ticker.change_pct_24h >= 0 ? '+' : ''}{ticker.change_pct_24h.toFixed(2)}%
              </span>
            </div>
            <Stat label="24h High"  value={`$${ticker.high_24h.toLocaleString()}`} />
            <Stat label="24h Low"   value={`$${ticker.low_24h.toLocaleString()}`} />
            <Stat label="24h Vol"   value={`$${(ticker.volume_24h / 1_000_000).toFixed(2)}M`} />
            <Stat label="OI"        value={`$${(ticker.open_interest / 1_000_000).toFixed(1)}M`} />
            <Stat label="Funding"   value={`${(ticker.funding_rate * 100).toFixed(4)}%`}
                  vClass={ticker.funding_rate >= 0 ? 'text-green-400' : 'text-red-400'} />
          </>
        )}
        {!connected && (
          <span className="ml-auto text-xs text-yellow-500 flex items-center gap-1">
            <span className="w-1.5 h-1.5 rounded-full bg-yellow-500 animate-pulse" /> Reconnecting…
          </span>
        )}
      </div>

      {/* Sub-header / Ticker bar */}
      <div className="flex items-center h-12 px-4 border-b border-[#14151f] bg-[#0a0b0f] gap-8 shrink-0">
        <div className="flex flex-col">
          <div className="flex items-center gap-2">
            <span className="text-sm font-semibold text-white">{activeMarket}</span>
            <span className="text-[10px] px-1.5 py-0.5 bg-blue-500/10 text-blue-400 rounded">Perp</span>
          </div>
          <span className="text-[10px] text-[#4b5068]">Solana / Pyth Network</span>
        </div>

        <div className="h-6 w-px bg-[#14151f]" />

        <div className="flex items-center gap-10">
          <Stat label="Index Price" value={`$${ticker?.price?.toLocaleString() || '---'}`} vClass="text-blue-400 font-medium" />
          <Stat label="24h Change" value={ticker ? `${ticker.change_pct_24h > 0 ? '+' : ''}${ticker.change_pct_24h.toFixed(2)}%` : '---'} 
            vClass={ticker ? (ticker.change_pct_24h > 0 ? 'text-green-400' : 'text-red-400') : 'text-[#4b5068]'} />
          <Stat label="24h High" value={`$${ticker?.high_24h?.toLocaleString() || '---'}`} />
          <Stat label="24h Low" value={`$${ticker?.low_24h?.toLocaleString() || '---'}`} />
          <Stat label="Funding Rate" value={ticker ? `${(ticker.funding_rate * 100).toFixed(4)}%` : '---'} vClass="text-yellow-500/80" />
        </div>
      </div>

      <PrivacyBanner />

      <div className="flex flex-1 min-h-0">
        {/* Chart + positions */}
        <div className="flex flex-col flex-1 min-w-0">
          <div className="flex items-center gap-1 px-3 py-1.5 border-b border-[#14151f] bg-[#0a0b0f] shrink-0">
            {(['1m','5m','15m','1h','4h','1d'] as const).map(i => (
              <button key={i} onClick={() => setChartInterval(i)}
                className={`px-3 py-1 text-[11px] rounded transition-all duration-200 ${
                  chartInterval === i ? 'bg-[#1e2130] text-white shadow-sm' : 'text-[#4b5068] hover:text-[#8b90a8]'
                }`}>
                {i}
              </button>
            ))}
          </div>
          <div className="flex-1 min-h-0 bg-[#060709]">
            <CandleChart marketId={activeMarket} interval={chartInterval} />
          </div>
          <div className="h-64 border-t border-[#14151f] flex flex-col shrink-0">
            <div className="flex border-b border-[#14151f] bg-[#0a0b0f] shrink-0">
              {(['positions','orders','history'] as const).map(tab => (
                <button key={tab} onClick={() => setBottomPanel(tab)}
                  className={`px-5 py-2.5 text-[11px] uppercase tracking-wider transition-all border-b-2 ${
                    bottomPanel === tab ? 'border-blue-500 text-white bg-blue-500/5' : 'border-transparent text-[#4b5068] hover:text-[#8b90a8]'
                  }`}>
                  {tab === 'positions' ? 'Open Positions' : tab === 'orders' ? 'Open Orders' : 'Trade History'}
                </button>
              ))}
            </div>
            <div className="flex-1 overflow-auto bg-[#0a0b0f]">
              <PositionsPanel view={bottomPanel} />
            </div>
          </div>
        </div>

        {/* Book / Trades / Polymarket */}
        <div className="w-80 border-l border-[#14151f] flex flex-col shrink-0 bg-[#0a0b0f]">
          <div className="flex border-b border-[#14151f] shrink-0">
            {([{k:'orderbook',l:'Order Book'},{k:'trades',l:'Recent Trades'},{k:'polymarket',l:'⬡ Polymarket'}] as const).map(({k,l}) => (
              <button key={k} onClick={() => setRightPanel(k as RightPanel)}
                className={`flex-1 py-3 text-[10px] uppercase tracking-widest border-b-2 transition-all ${
                  rightPanel === k ? 'border-blue-500 text-white bg-blue-500/5' : 'border-transparent text-[#4b5068] hover:text-[#8b90a8]'
                }`}>
                {l}
              </button>
            ))}
          </div>
          <div className="flex-1 overflow-hidden">
            {rightPanel === 'orderbook'  && <OrderBook marketId={activeMarket} />}
            {rightPanel === 'trades'     && <TradesPanel marketId={activeMarket} />}
            {rightPanel === 'polymarket' && <PolymarketPanel />}
          </div>
        </div>

        {/* Order entry */}
        <div className="w-72 border-l border-[#14151f] shrink-0 bg-[#0a0b0f]">
          <OrderPanel marketId={activeMarket} />
        </div>
      </div>
    </div>
  )
}

function Stat({ label, value, vClass = 'text-white' }: { label: string; value: string; vClass?: string }) {
  return (
    <div className="flex flex-col gap-0.5">
      <span className="text-[9px] text-[#4b5068] font-semibold uppercase tracking-[0.1em]">{label}</span>
      <span className={`text-[12px] font-medium tracking-tight ${vClass}`}>{value}</span>
    </div>
  )
}
