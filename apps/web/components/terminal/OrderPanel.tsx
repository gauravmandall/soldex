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
    <div className="flex flex-col h-full text-xs font-mono bg-[#0e1018]">
      {/* Market / Limit toggle */}
      <div className="flex border-b border-[#1e2130] shrink-0">
        {(['market', 'limit'] as const).map(m => (
          <button key={m} onClick={() => setMode(m)}
            className={`flex-1 py-2.5 capitalize border-b-2 transition-colors ${
              mode === m ? 'border-blue-500 text-white' : 'border-transparent text-[#4b5068] hover:text-[#8b90a8]'
            }`}>
            {m}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-auto p-3 flex flex-col gap-3">
        {/* Long / Short */}
        <div className="grid grid-cols-2 gap-1">
          <button onClick={() => setSide('long')}
            className={`py-2.5 rounded font-semibold transition-all ${
              side === 'long' ? 'btn-buy' : 'bg-[#13151d] text-[#4b5068] hover:text-[#8b90a8]'
            }`}>
            Long
          </button>
          <button onClick={() => setSide('short')}
            className={`py-2.5 rounded font-semibold transition-all ${
              side === 'short' ? 'btn-sell' : 'bg-[#13151d] text-[#4b5068] hover:text-[#8b90a8]'
            }`}>
            Short
          </button>
        </div>

        {/* Available */}
        <div className="flex justify-between text-[10px]">
          <span className="text-[#4b5068]">Available</span>
          <span className="text-[#e2e4ef] num">${balance.toLocaleString('en-US', { minimumFractionDigits: 2 })} USDC</span>
        </div>

        {/* Size */}
        <div>
          <label className="text-[10px] text-[#4b5068] uppercase tracking-wider block mb-1">Size</label>
          <div className="flex">
            <input
              type="number"
              min="0"
              placeholder="0.00"
              value={size}
              onChange={e => setSize(e.target.value)}
              className="flex-1 bg-[#13151d] border border-[#1e2130] rounded-l px-3 py-2 text-right num text-[#e2e4ef] outline-none focus:border-blue-500/60 transition-colors"
            />
            <span className="bg-[#1e2130] border border-l-0 border-[#1e2130] rounded-r px-3 py-2 text-[#8b90a8] flex items-center">
              {marketId.split('-')[0]}
            </span>
          </div>
        </div>

        {/* Limit price (only in limit mode) */}
        {mode === 'limit' && (
          <div>
            <label className="text-[10px] text-[#4b5068] uppercase tracking-wider block mb-1">Limit Price</label>
            <div className="flex">
              <input
                type="number"
                min="0"
                placeholder={ticker?.price.toFixed(2) ?? '0.00'}
                value={price}
                onChange={e => setPrice(e.target.value)}
                className="flex-1 bg-[#13151d] border border-[#1e2130] rounded-l px-3 py-2 text-right num text-[#e2e4ef] outline-none focus:border-blue-500/60 transition-colors"
              />
              <span className="bg-[#1e2130] border border-l-0 border-[#1e2130] rounded-r px-3 py-2 text-[#8b90a8] flex items-center">USD</span>
            </div>
          </div>
        )}

        {/* Leverage */}
        <div>
          <div className="flex justify-between mb-1">
            <label className="text-[10px] text-[#4b5068] uppercase tracking-wider">Leverage</label>
            <span className="text-blue-400 font-semibold">{leverage}x</span>
          </div>
          <input
            type="range" min={1} max={20} step={1}
            value={leverage}
            onChange={e => setLeverage(Number(e.target.value))}
            className="w-full accent-blue-500"
          />
          <div className="flex gap-1 mt-1">
            {LEVERAGES.map(l => (
              <button key={l} onClick={() => setLeverage(l)}
                className={`flex-1 py-0.5 rounded text-[10px] border transition-colors ${
                  leverage === l ? 'border-blue-500 text-blue-400' : 'border-[#1e2130] text-[#4b5068] hover:text-[#8b90a8]'
                }`}>
                {l}x
              </button>
            ))}
          </div>
        </div>

        {/* Slippage (market mode) */}
        {mode === 'market' && (
          <div>
            <label className="text-[10px] text-[#4b5068] uppercase tracking-wider block mb-1">Slippage Tolerance</label>
            <div className="flex gap-1">
              {[0.1, 0.5, 1].map(s => (
                <button key={s} onClick={() => setSlippage(s)}
                  className={`flex-1 py-1 rounded text-[10px] border transition-colors ${
                    slippage === s ? 'border-blue-500 text-blue-400' : 'border-[#1e2130] text-[#4b5068]'
                  }`}>
                  {s}%
                </button>
              ))}
              <button className="flex-1 py-1 rounded text-[10px] border border-[#1e2130] text-[#4b5068]">Custom</button>
            </div>
          </div>
        )}

        {/* Order summary */}
        <div className="bg-[#13151d] rounded border border-[#1e2130] p-3 flex flex-col gap-1.5">
          <Row label="Order Value"  value={`$${(parseFloat(size || '0') * (ticker?.price ?? 0)).toFixed(2)}`} />
          <Row label="Collateral"   value={`$${collateralUsd.toFixed(2)}`} />
          <Row label="Taker Fee (0.05%)" value={`$${takerFee.toFixed(4)}`} />
          <Row label="Liq. Price"   value={`$${liqPrice.toFixed(2)}`} vClass="text-yellow-400" />
          <div className="border-t border-[#1e2130] pt-1.5 mt-0.5">
            <Row label="Total Cost" value={`$${(collateralUsd + takerFee).toFixed(2)}`} vClass="text-white font-semibold" />
          </div>
        </div>

        {/* Submit */}
        <button
          onClick={handleSubmit}
          disabled={loading || !size}
          className={`py-3 rounded font-bold text-sm transition-all disabled:opacity-40 ${
            side === 'long' ? 'btn-buy' : 'btn-sell'
          }`}>
          {loading ? 'Submitting…' : `${side === 'long' ? 'Long' : 'Short'} ${marketId.split('-')[0]}`}
        </button>
      </div>
    </div>
  )
}

function Row({ label, value, vClass = 'text-[#e2e4ef]' }: { label: string; value: string; vClass?: string }) {
  return (
    <div className="flex justify-between">
      <span className="text-[#4b5068]">{label}</span>
      <span className={`num ${vClass}`}>{value}</span>
    </div>
  )
}
