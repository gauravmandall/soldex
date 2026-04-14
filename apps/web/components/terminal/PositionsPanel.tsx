'use client'
import React from 'react'
import { useMarketStore } from '@/hooks/useMarketStore'
import { useSelfCustodyWallet } from '@/hooks/useSelfCustodyWallet'

export function PositionsPanel({ view }: { view: 'positions' | 'orders' | 'history' }) {
  const { positions, openOrders } = useMarketStore()
  const { wallet } = useSelfCustodyWallet()

  if (!wallet) return (
    <div className="flex items-center justify-center h-full text-[#4a5568] text-[11px] font-bold uppercase tracking-widest">
      Connect wallet to view status
    </div>
  )

  if (view === 'positions') return (
    <table className="w-full text-[11px] font-mono border-collapse">
      <thead>
        <tr className="text-[10px] text-[#4a5568] font-bold uppercase tracking-wider border-b border-[#1e2634] bg-[#0d1117]">
          <Th>Market</Th><Th>Side</Th><Th>Size</Th><Th>Entry</Th><Th>Mark</Th><Th>PnL</Th><Th>Liq.</Th>
        </tr>
      </thead>
      <tbody className="bg-[#0a0e15]">
        {positions.map((p, i) => (
          <tr key={i} className="border-b border-white/[0.03] hover:bg-[#111827] transition-colors group">
            <Td className="font-bold text-[#e2e8f0]">{p.market_id}</Td>
            <Td>
              <span className={`px-1.5 py-0.5 rounded-[3px] font-bold ${
                p.side === 'long' ? 'bg-[#064e3b] text-[#34d399]' : 'bg-[#4c0519] text-[#fb7185]'
              }`}>
                {p.side.toUpperCase()}
              </span>
            </Td>
            <Td className="num text-[#94a3b8]">{p.size}</Td>
            <Td className="num text-[#94a3b8]">${p.entry_price.toLocaleString(undefined, { minimumFractionDigits: 2 })}</Td>
            <Td className="num text-[#94a3b8]">${p.mark_price.toLocaleString(undefined, { minimumFractionDigits: 2 })}</Td>
            <Td className={`num font-bold ${p.unrealized_pnl >= 0 ? 'text-[#10b981]' : 'text-[#ef4444]'}`}>
              {p.unrealized_pnl >= 0 ? '▲' : '▼'} ${Math.abs(p.unrealized_pnl).toFixed(2)}
            </Td>
            <Td className="num text-[#f59e0b] font-semibold">${p.liquidation_price.toFixed(2)}</Td>
          </tr>
        ))}
        {positions.length === 0 && (
          <tr>
            <td colSpan={7} className="text-center py-12">
              <div className="flex flex-col items-center gap-2">
                <div className="w-8 h-8 rounded-full bg-[#111827] flex items-center justify-center text-[#4a5568]">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M12 2v20M17 5l-5-5-5 5M17 19l-5 5-5-5"/></svg>
                </div>
                <span className="text-[#4a5568] font-bold uppercase tracking-widest text-[10px]">No Open Positions</span>
              </div>
            </td>
          </tr>
        )}
      </tbody>
    </table>
  )

  return (
    <div className="flex flex-col items-center justify-center h-full gap-2 opacity-40">
       <span className="text-[#4a5568] font-bold uppercase tracking-widest text-[10px]">No {view} found</span>
    </div>
  )
}

const Th = ({ children }: { children: React.ReactNode }) => (
  <th className="px-4 py-2.5 text-left font-bold">{children}</th>
)
const Td = ({ children, className = '' }: { children: React.ReactNode; className?: string }) => (
  <td className={`px-4 py-3 ${className}`}>{children}</td>
)
