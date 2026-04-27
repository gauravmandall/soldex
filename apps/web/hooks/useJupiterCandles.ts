'use client'

import { useEffect, useState, useCallback, useRef } from 'react'
import { UTCTimestamp } from 'lightweight-charts'

/**
 * useJupiterCandles
 * ─────────────────────────────────────────────────────────────────────────────
 * Manages OHLC data for the candlestick chart using Pyth Network (Solana Native).
 */

interface Candle {
  time: UTCTimestamp
  open: number
  high: number
  low: number
  close: number
  volume?: number
}

const PYTH_TV_URL = 'https://benchmarks.pyth.network/v1/shims/tradingview/history'

const MARKET_TO_PYTH: Record<string, string> = {
  'SOL-USDC': 'Crypto.SOL/USD',
  'BTC-USDC': 'Crypto.BTC/USD',
  'ETH-USDC': 'Crypto.ETH/USD',
  'JUP-USDC': 'Crypto.JUP/USD',
}

const INTERVAL_TO_PYTH: Record<string, string> = {
  '1m': '1',
  '5m': '5',
  '15m': '15',
  '30m': '30',
  '1h': '60',
  '4h': '240',
  '1d': '1D',
}

export function useJupiterCandles(marketId: string, interval: string) {
  const [data, setData] = useState<Candle[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const fetchHistory = useCallback(async () => {
    const symbol = MARKET_TO_PYTH[marketId]
    const resolution = INTERVAL_TO_PYTH[interval]
    if (!symbol || !resolution) return

    setLoading(true)
    setError(null)
    
    try {
      const to = Math.floor(Date.now() / 1000)
      const from = to - (getSeconds(interval) * 500)
      
      const url = `${PYTH_TV_URL}?symbol=${symbol}&resolution=${resolution}&from=${from}&to=${to}`
      const response = await fetch(url)
      const json = await response.json()

      if (json.s === 'ok') {
        const candles: Candle[] = json.t.map((t: number, i: number) => ({
          time: t as UTCTimestamp,
          open: json.o[i],
          high: json.h[i],
          low: json.l[i],
          close: json.c[i],
          volume: json.v ? json.v[i] : 0,
        }))

        setData(candles)
      } else {
        throw new Error('No history available from Pyth')
      }
    } catch (err) {
      console.error('Failed to fetch Pyth history:', err)
      setError(err instanceof Error ? err.message : 'Unknown error')
    } finally {
      setLoading(false)
    }
  }, [marketId, interval])

  useEffect(() => {
    fetchHistory()
    // In a real implementation, we would subscribe to Pyth benchmarks WS for real-time updates.
    // For now, we rely on the ticker updates from our engine to update the last candle.
  }, [marketId, interval, fetchHistory])

  return { data, loading, error }
}

function getSeconds(interval: string): number {
  const num = parseInt(interval)
  if (interval.endsWith('m')) return num * 60
  if (interval.endsWith('h')) return num * 3600
  if (interval.endsWith('d')) return num * 86400
  return 60
}
