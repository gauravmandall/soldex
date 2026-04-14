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

export const MARKETS = ['SOL-USDC', 'BTC-USDC', 'ETH-USDC', 'JUP-USDC', 'WIF-USDC', 'PYTH-USDC']
export type MarketId = typeof MARKETS[number] | string
export type RightPanel = 'orderbook' | 'trades' | 'polymarket'
export type BottomPanel = 'positions' | 'orders' | 'history'

interface Props { defaultMarket?: string }

export function TradingTerminal({ defaultMarket = 'SOL-USDC' }: Props) {
  const { ticker, setActiveMarket, activeMarket, polyOpportunities } = useMarketStore()
  const { connected } = useEngineWS()
  const [rightPanel, setRightPanel] = useState<RightPanel>('orderbook')
  const [bottomPanel, setBottomPanel] = useState<BottomPanel>('positions')
  const [chartInterval, setChartInterval] = useState<'1m'|'5m'|'15m'|'1h'|'4h'|'1d'>('30m')

  useEffect(() => { setActiveMarket(defaultMarket) }, [defaultMarket, setActiveMarket])

  const PAIRS = [
    { symbol: 'SOL-USDC', price: ticker?.market_id === 'SOL-USDC' ? ticker.price : 85.83, change: 4.72 },
    { symbol: 'BTC-USDC', price: 84312.5, change: -0.23 },
    { symbol: 'ETH-USDC', price: 2312.4, change: -0.65 },
  ]

  const pxColor = ticker
    ? (ticker.change_pct_24h >= 0 ? 'text-[#10b981]' : 'text-[#ef4444]')
    : 'text-[#94a3b8]'

  return (
    <div className="flex flex-col h-screen w-screen bg-[#0d1117] overflow-hidden text-[#e2e8f0] font-sans selection:bg-blue-500/30">
      <TopBar connected={connected} />
      <PrivacyBanner />

      {/* PAIRS TICKER STRIP */}
      <div className="flex items-center h-[34px] border-b border-[#1e2634] bg-[#0a0e15] overflow-x-auto no-scrollbar shrink-0">
        {PAIRS.map((p) => (
          <div key={p.symbol} 
            onClick={() => setActiveMarket(p.symbol)}
            className={`flex items-center gap-2.5 px-4 h-full cursor-pointer border-r border-[#1e2634] transition-colors whitespace-nowrap ${
              activeMarket === p.symbol ? 'bg-[#111827] border-b-2 border-b-[#3b82f6]' : 'hover:bg-[#111827]'
            }`}>
            <span className="text-[10px] font-medium text-[#94a3b8]">{p.symbol}</span>
            <span className="text-[11px] font-mono font-600">{p.price.toLocaleString(undefined, { minimumFractionDigits: p.price < 10 ? 3 : 1 })}</span>
            <span className={`text-[9px] ${p.change >= 0 ? 'text-[#10b981]' : 'text-[#ef4444]'}`}>
              {p.change >= 0 ? '+' : ''}{p.change}%
            </span>
          </div>
        ))}
        {/* POLYMARKET INDICATOR */}
        <div className="flex items-center h-full px-4 gap-4 overflow-hidden bg-[#0d121c]/50">
          {polyOpportunities.slice(0, 1).map((opp, i) => (
            <div key={i} className="flex items-center gap-2 group cursor-pointer" onClick={() => setRightPanel('polymarket')}>
              <div className="text-[9px] font-bold px-1.5 py-0.5 bg-purple-500/10 text-purple-400 rounded border border-purple-500/20 uppercase">Opportunity</div>
              <span className="text-[10px] text-[#8b90a8] group-hover:text-[#e2e8f0] transition-colors truncate max-w-[300px]">{opp.question}</span>
              <span className="text-[10px] font-mono font-bold text-[#3b82f6]">{(opp.yes_price * 100).toFixed(0)}% YES</span>
            </div>
          ))}
          {polyOpportunities.length > 1 && (
            <span className="text-[9px] text-[#4b5563] font-bold">+{polyOpportunities.length - 1} MORE</span>
          )}
        </div>
      </div>

      {/* MAIN MARKET HEADER */}
      <div className="flex items-center h-[56px] px-4 border-b border-[#1e2634] bg-[#0d1117] gap-6 shrink-0">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-full bg-gradient-to-br from-[#9945FF] to-[#14F195] flex items-center justify-center text-[10px] font-bold text-white shadow-lg">
            {activeMarket?.split('-')[0]}
          </div>
          <div className="flex flex-col gap-0.5">
            <div className="flex items-center gap-2">
              <MarketSelector />
              <span className="text-[9px] px-1.5 py-0.5 bg-[#1e3a5f] text-[#60a5fa] rounded font-bold uppercase">20x</span>
            </div>
            <div className="flex items-center gap-1.5">
              <span className={`text-lg font-bold font-mono num leading-none ${pxColor}`}>
                {ticker?.price.toLocaleString(undefined, { minimumFractionDigits: 3 }) || '---'}
              </span>
              <span className={`text-[10px] font-bold num ${pxColor}`}>
                {ticker ? (ticker.change_pct_24h >= 0 ? '+' : '') : ''}{ticker?.change_pct_24h.toFixed(2)}%
              </span>
            </div>
          </div>
        </div>

        <div className="h-8 w-px bg-[#1e2634] mx-2" />

        <div className="flex items-center gap-8 overflow-hidden">
          <Stat label="Index Price" value={`$${ticker?.price?.toLocaleString(undefined, { minimumFractionDigits: 3 }) || '---'}`} vClass="text-[#60a5fa] font-semibold" />
          <Stat label="24h High"     value={`$${ticker?.high_24h?.toLocaleString(undefined, { minimumFractionDigits: 2 }) || '---'}`} />
          <Stat label="24h Low"      value={`$${ticker?.low_24h?.toLocaleString(undefined, { minimumFractionDigits: 2 }) || '---'}`} />
          <Stat label="24h Volume"   value={`$${ticker ? (ticker.volume_24h / 1_000_000).toFixed(2) : '---'}M`} />
          <Stat label="Open Interest" value={`$${ticker ? (ticker.open_interest / 1_000_000).toFixed(1) : '---'}M`} />
          <div className="flex flex-col gap-0.5">
            <span className="text-[9px] text-[#4a5568] font-bold uppercase tracking-wider">Funding / Countdown</span>
            <div className="flex items-baseline gap-2">
              <span className="text-[11px] font-mono text-[#10b981] font-semibold">
                {ticker ? `${(ticker.funding_rate * 100).toFixed(4)}%` : '---'}
              </span>
              <span className="text-[10px] font-mono text-[#94a3b8]">00:36:39</span>
            </div>
          </div>
        </div>

        {!connected && (
          <div className="ml-auto flex items-center gap-2 px-3 py-1.5 rounded bg-red-500/10 border border-red-500/20">
             <div className="w-1.5 h-1.5 rounded-full bg-red-400 animate-pulse" />
             <span className="text-[10px] text-red-400 font-bold uppercase letter-spacing-[0.05em]">Disconnected</span>
          </div>
        )}
      </div>

      <div className="flex flex-1 min-h-0 bg-[#0d1117]">
        {/* CHART AREA */}
        <div className="flex flex-col flex-1 min-w-0 border-r border-[#1e2634]">
          <div className="flex items-center justify-between px-3 h-[38px] border-b border-[#1e2634] bg-[#0d1117] shrink-0">
            <div className="flex items-center gap-1">
              {(['1m','5m','15m','1h','4h','1d'] as const).map(i => (
                <button key={i} onClick={() => setChartInterval(i)}
                  className={`px-2.5 py-1 text-[10px] font-bold uppercase rounded transition-all ${
                    chartInterval === i ? 'bg-[#1e2634] text-[#60a5fa] shadow-inner' : 'text-[#64748b] hover:text-[#8b90a8]'
                  }`}>
                  {i}
                </button>
              ))}
            </div>
            <div className="flex items-center gap-3">
              <button className="text-[#64748b] hover:text-[#e2e8f0] transition-colors"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M12 20v-6M12 10V4M8 17l4 4 4-4M8 7l4-4 4 4"/></svg></button>
              <button className="text-[#64748b] hover:text-[#e2e8f0] transition-colors"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg></button>
            </div>
          </div>
          <div className="flex-1 min-h-0 bg-[#0d1117] p-1">
            <CandleChart marketId={activeMarket} interval={chartInterval} />
          </div>

          {/* POSITIONS AREA */}
          <div className="h-[240px] border-t border-[#1e2634] flex flex-col shrink-0 bg-[#0a0e15]">
            <div className="flex border-b border-[#1e2634] shrink-0">
              {([['positions','Positions'],['orders','Open Orders'],['history','Order History']] as const).map(([tab, label]) => (
                <button key={tab} onClick={() => setBottomPanel(tab)}
                  className={`px-6 py-2.5 text-[10px] font-bold uppercase tracking-wider transition-all border-b-2 ${
                    bottomPanel === tab ? 'border-[#3b82f6] text-[#e2e8f0] bg-[#111827]' : 'border-transparent text-[#64748b] hover:text-[#8b90a8]'
                  }`}>
                  {label}
                </button>
              ))}
            </div>
            <div className="flex-1 overflow-auto">
              <PositionsPanel view={bottomPanel} />
            </div>
          </div>
        </div>

        {/* RIGHT PANELS */}
        <div className="flex w-[540px] shrink-0">
          {/* BOOK / TRADES / POLY */}
          <div className="w-[280px] border-r border-[#1e2634] flex flex-col bg-[#0a0e15]">
            <div className="flex border-b border-[#1e2634] shrink-0 overflow-x-auto no-scrollbar">
              {([['orderbook','Book'],['trades','Trades'],['polymarket','Polymarket']] as const).map(([k,l]) => (
                <button key={k} onClick={() => setRightPanel(k as RightPanel)}
                  className={`flex-1 py-3 px-2 text-[10px] font-bold uppercase tracking-widest border-b-2 transition-all whitespace-nowrap ${
                    rightPanel === k ? 'border-[#3b82f6] text-[#e2e8f0] bg-[#111827]' : 'border-transparent text-[#64748b] hover:text-[#8b90a8]'
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

          {/* ORDER PANEL */}
          <div className="w-[260px] bg-[#0d1117]">
            <OrderPanel marketId={activeMarket} />
          </div>
        </div>
      </div>
    </div>
  )
}

function Stat({ label, value, vClass = 'text-[#e2e8f0]' }: { label: string; value: string; vClass?: string }) {
  return (
    <div className="flex flex-col gap-0.5 whitespace-nowrap">
      <span className="text-[9px] text-[#4a5568] font-bold uppercase tracking-wider leading-none">{label}</span>
      <span className={`text-[11px] font-mono font-semibold tracking-tight ${vClass}`}>{value}</span>
    </div>
  )
}
