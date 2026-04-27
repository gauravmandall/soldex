'use client'

import React, { useState, useCallback } from 'react'
import { useMarketStore } from '@/hooks/useMarketStore'
import { useSelfCustodyWallet } from '@/hooks/useSelfCustodyWallet'
import { useEngineWS } from '@/hooks/useEngineWS'
import toast from 'react-hot-toast'

type OrderMode = 'market' | 'limit'
type Side = 'long' | 'short'

interface Props { marketId: string }

export function OrderPanel({ marketId }: Props) {
  const { ticker } = useMarketStore()
  const { wallet, balance } = useSelfCustodyWallet()
  const { sendMessage } = useEngineWS()

  const [mode, setMode] = useState<OrderMode>('market')
  const [side, setSide] = useState<Side>('long')
  const [size, setSize] = useState('')
  const [price, setPrice] = useState('')
  const [leverage, setLeverage] = useState(5)
  const [slippage, setSlippage] = useState(0.5)
  const [loading, setLoading] = useState(false)

  const collateralUsd = parseFloat(size || '0') * (ticker?.price ?? 0) / leverage
  const takerFee = parseFloat(size || '0') * (ticker?.price ?? 0) * 0.0005
  const liqPrice = ticker
    ? side === 'long'
      ? ticker.price * (1 - 1 / leverage + 0.025)
      : ticker.price * (1 + 1 / leverage - 0.025)
    : 0

  const handleSubmit = useCallback(async () => {
    if (!wallet) {
      toast.error('Connect or generate a wallet first')
      return
    }
    if (!size || parseFloat(size) <= 0) {
      toast.error('Enter a valid size')
      return
    }

    setLoading(true)
    try {
      sendMessage({
        type: 'build_perps_order',
        market_id: marketId,
        side,
        size: parseFloat(size),
        leverage,
        collateral_usdc: collateralUsd,
        owner_pubkey: wallet.pubkey,
      })
      toast.success(`${side.toUpperCase()} order submitted`)
    } catch (e) {
      toast.error('Order failed')
    } finally {
      setLoading(false)
    }
  }, [wallet, size, side, leverage, collateralUsd, marketId, sendMessage])

  const LEVERAGES = [2, 5, 10, 20]

  return (
    <div className="flex flex-col h-full bg-[#0d1117] border-l border-[#1e2634]">
      {/* Jupiter Indicator */}
      <div className="flex items-center gap-2 px-4 py-2 bg-[var(--blue)]/5 border-b border-[var(--bd)]">
        <div className="w-1.5 h-1.5 rounded-full bg-[var(--blue)] animate-pulse" />
        <span className="text-[9px] font-bold text-[var(--blue)] uppercase tracking-[2px]">Jupiter Perps v2</span>
      </div>

      {/* Mode Toggle */}
      <div className="flex border-b border-[#1e2634] shrink-0">
        {(['market', 'limit'] as const).map(m => (
          <button key={m} onClick={() => setMode(m)}
            className={`flex-1 py-3 text-[10px] font-bold uppercase tracking-wider border-b-2 transition-all duration-200 ${
              mode === m ? 'border-[#3b82f6] text-[#e2e8f0] bg-[#111827]' : 'border-transparent text-[#64748b] hover:text-[#8b90a8]'
            }`}>
            {m}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-auto p-4 flex flex-col gap-4">
        {/* Side Selection */}
        <div className="grid grid-cols-2 gap-2">
          <button onClick={() => setSide('long')}
            className={`py-2.5 rounded-md font-bold text-xs transition-all ${
              side === 'long' 
                ? 'bg-[#064e3b] text-[#34d399] border border-[#10b981]/30 shadow-[0_0_12px_rgba(16,185,129,0.1)]' 
                : 'bg-[#111827] text-[#64748b] border border-[#1e2634] hover:text-[#8b90a8]'
            }`}>
            Long / Buy
          </button>
          <button onClick={() => setSide('short')}
            className={`py-2.5 rounded-md font-bold text-xs transition-all ${
              side === 'short' 
                ? 'bg-[#4c0519] text-[#fb7185] border border-[#f43f5e]/30 shadow-[0_0_12px_rgba(244,63,94,0.1)]' 
                : 'bg-[#111827] text-[#64748b] border border-[#1e2634] hover:text-[#8b90a8]'
            }`}>
            Short / Sell
          </button>
        </div>

        {/* Available Balance */}
        <div className="flex justify-between items-center text-[10px] uppercase font-bold text-[#4a5568]">
          <span>Available</span>
          <span className="text-[#94a3b8] font-mono">${balance.toLocaleString(undefined, { minimumFractionDigits: 2 })} USDC</span>
        </div>

        {/* Leverage Slider */}
        <div className="flex flex-col gap-2">
          <div className="flex justify-between items-center">
            <span className="text-[10px] font-bold text-[#4a5568] uppercase">Leverage</span>
            <span className="text-[11px] font-mono font-bold text-[#3b82f6]">{leverage}x</span>
          </div>
          <input
            type="range" min={1} max={50} step={1}
            value={leverage}
            onChange={e => setLeverage(Number(e.target.value))}
            className="w-full h-1 bg-[#1e2634] rounded-lg appearance-none cursor-pointer accent-[#3b82f6]"
          />
          <div className="flex gap-1">
            {LEVERAGES.map(l => (
              <button key={l} onClick={() => setLeverage(l)}
                className={`flex-1 py-1 rounded text-[10px] font-bold border transition-colors ${
                  leverage === l ? 'bg-[#1e3a5f] border-[#3b82f6] text-[#60a5fa]' : 'bg-[#111827] border-[#1e2634] text-[#4a5568] hover:text-[#94a3b8]'
                }`}>
                {l}x
              </button>
            ))}
          </div>
        </div>

        {/* Size Input */}
        <div className="flex flex-col gap-1.5">
          <label className="text-[10px] font-bold text-[#4a5568] uppercase tracking-wider">Size (SOL)</label>
          <div className="relative group">
            <input
              type="number"
              min="0"
              placeholder="0.00"
              value={size}
              onChange={e => setSize(e.target.value)}
              className="w-full bg-[#111827] border border-[#1e2634] rounded-md px-3 py-2 text-right font-mono text-sm text-[#e2e8f0] outline-none group-hover:border-[#3b82f6]/40 focus:border-[#3b82f6] transition-all"
            />
            <span className="absolute left-3 top-1/2 -translate-y-1/2 text-[10px] font-bold text-[#4a5568] group-focus-within:text-[#64748b]">SOL</span>
          </div>
        </div>

        {/* Limit Price */}
        {mode === 'limit' && (
          <div className="flex flex-col gap-1.5">
            <label className="text-[10px] font-bold text-[#4a5568] uppercase tracking-wider">Limit Price</label>
            <div className="relative group">
              <input
                type="number"
                min="0"
                placeholder={ticker?.price.toFixed(3) ?? '0.000'}
                value={price}
                onChange={e => setPrice(e.target.value)}
                className="w-full bg-[#111827] border border-[#1e2634] rounded-md px-3 py-2 text-right font-mono text-sm text-[#e2e8f0] outline-none group-hover:border-[#3b82f6]/40 focus:border-[#3b82f6] transition-all"
              />
              <span className="absolute left-3 top-1/2 -translate-y-1/2 text-[10px] font-bold text-[#4a5568] group-focus-within:text-[#64748b]">USDC</span>
            </div>
          </div>
        )}

        <div className="h-px bg-[#1e2634] my-2" />

        {/* Order Details */}
        <div className="flex flex-col gap-2 bg-[#0a0e15] border border-[#1e2634] rounded-md p-3">
          <Row label="Order Value"  value={`$${(parseFloat(size || '0') * (ticker?.price ?? 0)).toFixed(2)}`} />
          <Row label="Collateral"   value={`$${collateralUsd.toFixed(2)}`} />
          <Row label="Taker Fee (0.05%)" value={`$${takerFee.toFixed(3)}`} />
          <Row label="Estimated PnL" value="---" vClass="text-[#64748b]" />
          <div className="h-px bg-[#1e2634] my-1" />
          <Row label="Total Cost" value={`$${(collateralUsd + takerFee).toFixed(2)}`} vClass="text-[#e2e8f0] font-bold" />
        </div>

        {/* Action Button */}
        <button
          onClick={handleSubmit}
          disabled={loading || !size}
          className={`w-full py-3.5 rounded-md font-bold text-sm transition-all disabled:opacity-40 disabled:cursor-not-allowed uppercase tracking-wider shadow-lg ${
            side === 'long' ? 'btn-buy' : 'btn-sell'
          }`}>
          {loading ? 'Processing…' : `${side === 'long' ? 'Buy / Long' : 'Sell / Short'} ${marketId.split('-')[0]}`}
        </button>

        <div className="flex flex-col gap-1.5 mt-2">
           <label className="flex items-center gap-2 text-[10px] font-bold text-[#4a5568] cursor-pointer">
             <input type="checkbox" className="accent-[#3b82f6]" /> Reduce Only
           </label>
           <label className="flex items-center gap-2 text-[10px] font-bold text-[#4a5568] cursor-pointer">
             <input type="checkbox" className="accent-[#f59e0b]" /> Take Profit / Stop Loss
           </label>
        </div>
      </div>
    </div>
  )
}

function Row({ label, value, vClass = 'text-[#94a3b8]' }: { label: string; value: string; vClass?: string }) {
  return (
    <div className="flex justify-between items-center text-[10px] uppercase font-bold tracking-tight">
      <span className="text-[#4a5568]">{label}</span>
      <span className={`font-mono text-[11px] ${vClass}`}>{value}</span>
    </div>
  )
}


// function Row({ label, value, vClass = 'text-[#e2e4ef]' }: { label: string; value: string; vClass?: string }) {
//   return (
//     <div className="flex justify-between">
//       <span className="text-[#4b5068]">{label}</span>
//       <span className={`num ${vClass}`}>{value}</span>
//     </div>
//   )
// }
