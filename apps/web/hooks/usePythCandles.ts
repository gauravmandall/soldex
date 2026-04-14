import { useEffect, useState, useCallback, useRef } from 'react'
import { useMarketStore } from './useMarketStore'
import { UTCTimestamp } from 'lightweight-charts'

/**
 * usePythCandles
 * ─────────────────────────────────────────────────────────────────────────────
 * Manages OHLC data for the candlestick chart by:
 * 1. Fetching historical data from Pyth Benchmarks API
 * 2. Listening for real-time price updates (TickerUpdate) from Soldex Engine
 * 3. Aggregating ticks into the current open candle
 */

interface Candle {
  time: UTCTimestamp
  open: number
  high: number
  low: number
  close: number
  volume?: number
}

const PYTH_BENCHMARKS_URL = 'https://benchmarks.pyth.network/v1/shims/tradingview/history'

const MARKET_TO_PYTH: Record<string, string> = {
  'SOL-PERP': 'Crypto.SOL/USD',
  'BTC-PERP': 'Crypto.BTC/USD',
  'ETH-PERP': 'Crypto.ETH/USD',
}

const INTERVAL_TO_RES: Record<string, string> = {
  '1m': '1',
  '5m': '5',
  '15m': '15',
  '1h': '60',
  '4h': '240',
  '1d': '1D',
}

export function usePythCandles(marketId: string, interval: string) {
  const [data, setData] = useState<Candle[]>([])
  const [loading, setLoading] = useState(true)
  const ticker = useMarketStore((s) => s.ticker)
  
  // Ref to track the current developing candle to avoid state thrashing
  const currentCandleRef = useRef<Candle | null>(null)
  const lastBarTimeRef = useRef<number>(0)

  const fetchHistory = useCallback(async () => {
    const symbol = MARKET_TO_PYTH[marketId]
    const resolution = INTERVAL_TO_RES[interval]
    if (!symbol || !resolution) return

    setLoading(true)
    try {
      const to = Math.floor(Date.now() / 1000)
      const from = to - (getSeconds(interval) * 200) // fetch 200 bars
      
      const url = `${PYTH_BENCHMARKS_URL}?symbol=${symbol}&resolution=${resolution}&from=${from}&to=${to}`
      const resp = await fetch(url)
      const json = await resp.json()

      if (json.s === 'ok') {
        const candles: Candle[] = json.t.map((t: number, i: number) => ({
          time: t as UTCTimestamp,
          open: json.o[i],
          high: json.h[i],
          low: json.l[i],
          close: json.c[i],
          volume: json.v[i],
        }))
        setData(candles)
        
        if (candles.length > 0) {
          const last = candles[candles.length - 1]
          currentCandleRef.current = { ...last }
          lastBarTimeRef.current = last.time as number
        }
      }
    } catch (err) {
      console.error('Failed to fetch Pyth history:', err)
    } finally {
      setLoading(false)
    }
  }, [marketId, interval])

  useEffect(() => {
    fetchHistory()
  }, [fetchHistory])

  // Process real-time ticker updates
  useEffect(() => {
    if (!ticker || ticker.market_id !== marketId || loading) return

    const price = ticker.price
    const timestamp = Math.floor(ticker.timestamp / 1000)
    const resSecs = getSeconds(interval)
    const barTime = Math.floor(timestamp / resSecs) * resSecs

    setData((prev) => {
      if (prev.length === 0) return prev

      const last = prev[prev.length - 1]
      
      if (barTime > last.time) {
        // New bar
        const newCandle: Candle = {
          time: barTime as UTCTimestamp,
          open: price,
          high: price,
          low: price,
          close: price,
        }
        currentCandleRef.current = newCandle
        lastBarTimeRef.current = barTime
        return [...prev, newCandle]
      } else {
        // Update existing bar
        const updatedCandle = {
          ...last,
          high: Math.max(last.high, price),
          low: Math.min(last.low, price),
          close: price,
        }
        currentCandleRef.current = updatedCandle
        
        const newArr = [...prev]
        newArr[newArr.length - 1] = updatedCandle
        return newArr
      }
    })
  }, [ticker, marketId, interval, loading])

  return { data, loading }
}

function getSeconds(interval: string): number {
  const num = parseInt(interval)
  if (interval.endsWith('m')) return num * 60
  if (interval.endsWith('h')) return num * 3600
  if (interval.endsWith('d')) return num * 86400
  return 60
}
