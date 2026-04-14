'use client'
import React from 'react'
import { useMarketStore } from '@/hooks/useMarketStore'
import { useSelfCustodyWallet } from '@/hooks/useSelfCustodyWallet'
import { useEngineWS } from '@/hooks/useEngineWS'
import { TopBar } from '@/components/terminal/TopBar'
import { TrendingUp, TrendingDown, Activity } from 'lucide-react'

export default function PortfolioPage() {
  const { positions } = useMarketStore()
  const { wallet, balance, history } = useSelfCustodyWallet()
  const { connected } = useEngineWS()

  const totalPnl = positions.reduce((a, p) => a + p.unrealized_pnl, 0)
  const totalNotional = positions.reduce((a, p) => a + p.size * p.mark_price, 0)

  return (
    <div className="flex flex-col h-screen bg-[#0a0b0f]">
      <TopBar connected={connected} />
      <div className="flex-1 overflow-auto p-6 max-w-5xl mx-auto w-full">
        <h1 className="text-xl font-bold text-white mb-6">Portfolio</h1>

        {/* Summary cards */}
        <div className="grid grid-cols-4 gap-3 mb-6">
          {[
            { label: 'USDC Balance', value: `$${balance.toFixed(2)}`, color: 'text-white' },
            { label: 'Open Positions', value: positions.length.toString(), color: 'text-blue-400' },
            { label: 'Unrealized PnL', value: `${totalPnl >= 0 ? '+' : ''}$${totalPnl.toFixed(2)}`, color: totalPnl >= 0 ? 'text-green-400' : 'text-red-400' },
            { label: 'Total Notional', value: `$${(totalNotional / 1000).toFixed(1)}K`, color: 'text-[#e2e4ef]' },
          ].map(({ label, value, color }) => (
            <div key={label} className="bg-[#13151d] border border-[#1e2130] rounded p-4">
              <p className="text-[10px] text-[#4b5068] uppercase tracking-wider mb-1">{label}</p>
              <p className={`text-xl font-bold num ${color}`}>{value}</p>
            </div>
          ))}
        </div>

        {/* Positions */}
        <div className="bg-[#13151d] border border-[#1e2130] rounded mb-4">
          <div className="px-4 py-3 border-b border-[#1e2130]">
            <h2 className="text-sm font-semibold text-white">Open Positions</h2>
          </div>
          {positions.length === 0 ? (
            <div className="py-12 text-center text-[#4b5068] text-sm">No open positions</div>
          ) : (
            <table className="w-full text-xs font-mono">
              <thead>
                <tr className="text-[10px] text-[#4b5068] uppercase border-b border-[#1e2130]">
                  {['Market','Side','Size','Entry','Mark','PnL','Liq.','Leverage'].map(h => (
                    <th key={h} className="px-4 py-2 text-left font-normal">{h}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {positions.map((p, i) => (
                  <tr key={i} className="border-t border-[#1e2130] hover:bg-[#1a1d28]">
                    <td className="px-4 py-3 text-white">{p.market_id}</td>
                    <td className="px-4 py-3"><span className={p.side === 'long' ? 'text-green-400' : 'text-red-400'}>{p.side.toUpperCase()}</span></td>
                    <td className="px-4 py-3 num">{p.size}</td>
                    <td className="px-4 py-3 num">${p.entry_price.toFixed(2)}</td>
                    <td className="px-4 py-3 num">${p.mark_price.toFixed(2)}</td>
                    <td className={`px-4 py-3 num font-semibold ${p.unrealized_pnl >= 0 ? 'text-green-400' : 'text-red-400'}`}>
                      {p.unrealized_pnl >= 0 ? '+' : ''}${p.unrealized_pnl.toFixed(2)}
                    </td>
                    <td className="px-4 py-3 num text-yellow-400">${p.liquidation_price.toFixed(2)}</td>
                    <td className="px-4 py-3 num text-blue-400">{p.leverage}x</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>

        {/* Wallet rotation history */}
        <div className="bg-[#13151d] border border-[#1e2130] rounded">
          <div className="px-4 py-3 border-b border-[#1e2130] flex items-center justify-between">
            <h2 className="text-sm font-semibold text-white">Privacy Rotation Timeline</h2>
            <span className="text-[10px] text-[#4b5068]">{history.length} rotation(s)</span>
          </div>
          <div className="p-4">
            {wallet && (
              <div className="flex items-center gap-3 mb-3">
                <div className="w-2 h-2 rounded-full bg-green-400 shrink-0" />
                <div>
                  <p className="text-xs text-green-400 font-mono">{wallet.pubkey.slice(0, 16)}…{wallet.pubkey.slice(-8)}</p>
                  <p className="text-[10px] text-[#4b5068]">Active · Created {new Date(wallet.createdAt).toLocaleDateString()}</p>
                </div>
              </div>
            )}
            {history.slice().reverse().map((h, i) => (
              <div key={i} className="flex items-center gap-3 mb-3 opacity-50">
                <div className="w-2 h-2 rounded-full bg-[#4b5068] shrink-0" />
                <div>
                  <p className="text-xs text-[#8b90a8] font-mono">{h.pubkey.slice(0, 16)}…{h.pubkey.slice(-8)}</p>
                  <p className="text-[10px] text-[#4b5068]">Retired {new Date(h.retiredAt).toLocaleDateString()}</p>
                </div>
              </div>
            ))}
            {!wallet && history.length === 0 && (
              <p className="text-xs text-[#4b5068] text-center py-4">No wallet history yet</p>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
