'use client'
import React from 'react'
import { useMarketStore } from '@/hooks/useMarketStore'
import { useSelfCustodyWallet } from '@/hooks/useSelfCustodyWallet'
import { useEngineWS } from '@/hooks/useEngineWS'
import { TopBar } from '@/components/terminal/TopBar'
import { TrendingUp, TrendingDown, Activity, Wallet, Clock, Shield, BarChart3 } from 'lucide-react'

export default function PortfolioPage() {
  const { positions } = useMarketStore()
  const { wallet, balance, history } = useSelfCustodyWallet()
  const { connected } = useEngineWS()

  const totalPnl = positions.reduce((a, p) => a + p.unrealized_pnl, 0)
  const totalNotional = positions.reduce((a, p) => a + p.size * p.mark_price, 0)

  return (
    <div className="flex flex-col h-screen bg-[var(--bg)] text-[var(--tx)] font-mono selection:bg-blue-500/30">
      <TopBar connected={connected} />
      
      <div className="flex-1 overflow-auto no-scrollbar">
        <div className="max-w-6xl mx-auto p-8 flex flex-col gap-8">
          <div className="flex items-center justify-between border-b border-[var(--bd)] pb-6">
            <div className="flex flex-col gap-1">
              <h1 className="text-[20px] font-bold text-white uppercase tracking-widest">Global Portfolio</h1>
              <p className="text-[10px] text-[var(--tx3)] uppercase tracking-tighter">Unified metrics across Perps and Predictions</p>
            </div>
            <div className="flex items-center gap-4">
               <div className="px-4 py-2 bg-[var(--bg2)] border border-[var(--bd)] flex flex-col items-end">
                  <span className="text-[9px] text-[var(--tx3)] uppercase font-bold tracking-widest leading-none mb-1">Total Balance</span>
                  <span className="text-[16px] font-bold text-[var(--green)]">${balance.toLocaleString(undefined, { minimumFractionDigits: 2 })}</span>
               </div>
            </div>
          </div>

          {/* Summary stats */}
          <div className="grid grid-cols-4 gap-4">
            {[
              { label: 'Active Positions', value: positions.length.toString(), color: 'text-white' },
              { label: 'Unrealized PnL', value: `${totalPnl >= 0 ? '+' : ''}$${totalPnl.toFixed(2)}`, color: totalPnl >= 0 ? 'text-[var(--green)]' : 'text-[var(--red)]' },
              { label: 'Margin Usage', value: `$${(totalNotional / 10).toFixed(2)}`, color: 'text-[var(--blue)]' },
              { label: 'Gas Credits', value: '1.42 MATIC', color: 'text-[var(--tx2)]' },
            ].map(({ label, value, color }) => (
              <div key={label} className="bg-[var(--bg1)] border border-[var(--bd)] p-5 flex flex-col gap-2 shadow-inner">
                <span className="text-[10px] text-[var(--tx3)] uppercase tracking-widest font-bold">{label}</span>
                <span className={`text-[20px] font-bold num ${color}`}>{value}</span>
              </div>
            ))}
          </div>

          <div className="grid grid-cols-3 gap-8">
            {/* Positions Table */}
            <div className="col-span-2 flex flex-col gap-4">
              <div className="h-[40px] border-b border-[var(--bd)] flex items-center justify-between px-2">
                <h2 className="text-[11px] font-bold text-[var(--tx2)] uppercase tracking-widest flex items-center gap-2">
                  <Activity className="w-4 h-4" /> Open Positions
                </h2>
              </div>
              <div className="bg-[var(--bg1)] border border-[var(--bd)] overflow-hidden">
                {positions.length === 0 ? (
                  <div className="py-20 text-center flex flex-col items-center gap-3 text-[var(--tx3)]">
                    <BarChart3 className="w-8 h-8 opacity-20" />
                    <span className="text-[10px] uppercase tracking-widest font-bold">No active positions</span>
                  </div>
                ) : (
                  <table className="w-full text-[10px] text-left">
                    <thead>
                      <tr className="text-[var(--tx3)] uppercase tracking-tighter bg-[var(--bg2)]/50">
                        {['Market','Side','Size','Entry','PnL','Lev'].map(h => (
                          <th key={h} className="px-4 py-3 font-bold border-b border-[var(--bd)]">{h}</th>
                        ))}
                      </tr>
                    </thead>
                    <tbody>
                      {positions.map((p, i) => (
                        <tr key={i} className="border-b border-[var(--bd)] hover:bg-[var(--bg2)]/50 transition-colors">
                          <td className="px-4 py-4 font-bold text-white">{p.market_id}</td>
                          <td className="px-4 py-4"><span className={`px-1.5 py-0.5 rounded-[2px] font-bold ${p.side === 'long' ? 'bg-[var(--green)]/10 text-[var(--green)]' : 'bg-[var(--red)]/10 text-[var(--red)]'}`}>{p.side.toUpperCase()}</span></td>
                          <td className="px-4 py-4 num text-[var(--tx2)]">{p.size}</td>
                          <td className="px-4 py-4 num text-[var(--tx2)]">${p.entry_price.toFixed(2)}</td>
                          <td className={`px-4 py-4 num font-bold ${p.unrealized_pnl >= 0 ? 'text-[var(--green)]' : 'text-[var(--red)]'}`}>
                            {p.unrealized_pnl >= 0 ? '+' : ''}${p.unrealized_pnl.toFixed(2)}
                          </td>
                          <td className="px-4 py-4 text-[var(--blue)] font-bold">{p.leverage}x</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                )}
              </div>
            </div>

            {/* Privacy History */}
            <div className="flex flex-col gap-4">
              <div className="h-[40px] border-b border-[var(--bd)] flex items-center justify-between px-2">
                <h2 className="text-[11px] font-bold text-[var(--tx2)] uppercase tracking-widest flex items-center gap-2">
                  <Shield className="w-4 h-4" /> Security Log
                </h2>
              </div>
              <div className="bg-[var(--bg1)] border border-[var(--bd)] p-5 flex flex-col gap-6">
                {wallet && (
                  <div className="flex gap-4">
                    <div className="w-1.5 h-1.5 rounded-full bg-[var(--green)] mt-1.5 animate-pulse shrink-0" />
                    <div className="flex flex-col gap-1">
                      <p className="text-[11px] text-[var(--tx)] font-bold break-all">{wallet.pubkey}</p>
                      <p className="text-[9px] text-[var(--green)] uppercase font-bold tracking-widest">Active Identity</p>
                    </div>
                  </div>
                )}
                
                <div className="w-full h-[1px] bg-[var(--bd)]" />

                <div className="flex flex-col gap-4">
                  {history.slice().reverse().map((h, i) => (
                    <div key={i} className="flex gap-4 opacity-40">
                      <div className="w-1.5 h-1.5 rounded-full bg-[var(--tx3)] mt-1.5 shrink-0" />
                      <div className="flex flex-col gap-1">
                        <p className="text-[10px] text-[var(--tx2)] font-mono truncate max-w-[200px]">{h.pubkey}</p>
                        <p className="text-[9px] text-[var(--tx3)] uppercase font-bold">Retired · {new Date(h.retiredAt).toLocaleDateString()}</p>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
