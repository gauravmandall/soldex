'use client'
import React from 'react'
import { useMarketStore } from '@/hooks/useMarketStore'
import { useSelfCustodyWallet } from '@/hooks/useSelfCustodyWallet'

export function PositionsPanel({ view }: { view: 'positions' | 'orders' | 'history' }) {
  const { positions, openOrders } = useMarketStore()
  const { wallet } = useSelfCustodyWallet()

  if (!wallet) return (
    <div className="flex items-center justify-center h-full text-[#4b5068] text-xs">
      Connect wallet to view positions
    </div>
  )

  if (view === 'positions') return (
    <table className="w-full text-xs font-mono">
      <thead><tr className="text-[10px] text-[#4b5068] uppercase">
        <Th>Market</Th><Th>Side</Th><Th>Size</Th><Th>Entry</Th><Th>Mark</Th><Th>PnL</Th><Th>Liq.</Th>
      </tr></thead>
      <tbody>
        {positions.map((p, i) => (
          <tr key={i} className="border-t border-[#1e2130] hover:bg-[#1a1d28]">
            <Td>{p.market_id}</Td>
            <Td><span className={p.side === 'long' ? 'text-green-400' : 'text-red-400'}>{p.side.toUpperCase()}</span></Td>
            <Td className="num">{p.size}</Td>
            <Td className="num">${p.entry_price.toFixed(2)}</Td>
            <Td className="num">${p.mark_price.toFixed(2)}</Td>
            <Td className={`num ${p.unrealized_pnl >= 0 ? 'text-green-400' : 'text-red-400'}`}>
              {p.unrealized_pnl >= 0 ? '+' : ''}${p.unrealized_pnl.toFixed(2)}
            </Td>
            <Td className="num text-yellow-400">${p.liquidation_price.toFixed(2)}</Td>
          </tr>
        ))}
        {positions.length === 0 && (
          <tr><td colSpan={7} className="text-center py-6 text-[#4b5068]">No open positions</td></tr>
        )}
      </tbody>
    </table>
  )

  return (
    <div className="flex items-center justify-center h-full text-[#4b5068] text-xs">No {view}</div>
  )
}

const Th = ({ children }: { children: React.ReactNode }) => (
  <th className="px-3 py-2 text-left font-normal">{children}</th>
)
const Td = ({ children, className = '' }: { children: React.ReactNode; className?: string }) => (
  <td className={`px-3 py-2 ${className}`}>{children}</td>
)
