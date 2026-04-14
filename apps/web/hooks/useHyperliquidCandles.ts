'use client'

import { useEffect, useState, useCallback, useRef } from 'react'
import { useMarketStore } from './useMarketStore'
import { UTCTimestamp } from 'lightweight-charts'

/**
 * useHyperliquidCandles
 * ─────────────────────────────────────────────────────────────────────────────
 * Manages OHLC data for the candlestick chart by:
 * 1. Fetching historical data from Hyperliquid Info API
 * 2. Listening for real-time candle updates from Hyperliquid WebSocket
 */

interface Candle {
  time: UTCTimestamp
  open: number
  high: number
  low: number
  close: number
  volume?: number
}

const HL_INFO_URL = 'https://api.hyperliquid.xyz/info'
const HL_WS_URL = 'wss://api.hyperliquid.xyz/ws'

const MARKET_TO_COIN: Record<string, string> = {
  'SOL-USDC': 'SOL',
  'BTC-USDC': 'BTC',
  'ETH-USDC': 'ETH',
}

export function useHyperliquidCandles(marketId: string, interval: string) {
  const [data, setData] = useState<Candle[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  
  const currentCandleRef = useRef<Candle | null>(null)
  const wsRef = useRef<WebSocket | null>(null)

  const fetchHistory = useCallback(async () => {
    const coin = MARKET_TO_COIN[marketId]
    if (!coin) return

    setLoading(true)
    setError(null)
    
    try {
      const endTime = Date.now()
      const startTime = endTime - (getSeconds(interval) * 500 * 1000) // fetch 500 bars
      
      const response = await fetch(HL_INFO_URL, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          type: 'candleSnapshot',
          req: {
            coin,
            interval,
            startTime,
            endTime,
          }
        })
      })

      const json = await response.json()
      if (Array.isArray(json)) {
        const candles: Candle[] = json.map((c: any) => ({
          time: (c.t / 1000) as UTCTimestamp,
          open: parseFloat(c.o),
          high: parseFloat(c.h),
          low: parseFloat(c.l),
          close: parseFloat(c.c),
          volume: parseFloat(c.v),
        })).sort((a, b) => (a.time as number) - (b.time as number))

        setData(candles)
        if (candles.length > 0) {
          currentCandleRef.current = candles[candles.length - 1]
        }
      } else {
        throw new Error('Invalid response format from Hyperliquid')
      }
    } catch (err) {
      console.error('Failed to fetch Hyperliquid history:', err)
      setError(err instanceof Error ? err.message : 'Unknown error')
    } finally {
      setLoading(false)
    }
  }, [marketId, interval])

  useEffect(() => {
    fetchHistory()

    const coin = MARKET_TO_COIN[marketId]
    if (!coin) return

    // Setup WebSocket for real-time updates
    const ws = new WebSocket(HL_WS_URL)
    wsRef.current = ws

    ws.onopen = () => {
      ws.send(JSON.stringify({
        method: 'subscribe',
        subscription: {
          type: 'candle',
          coin,
          interval,
        }
      }))
    }

    ws.onmessage = (event) => {
      const msg = JSON.parse(event.data)
      if (msg.channel === 'candle' && msg.data) {
        const c = msg.data
        if (c.s !== coin || c.i !== interval) return

        const newCandle: Candle = {
          time: (c.t / 1000) as UTCTimestamp,
          open: parseFloat(c.o),
          high: parseFloat(c.h),
          low: parseFloat(c.l),
          close: parseFloat(c.c),
          volume: parseFloat(c.v),
        }

        setData((prev) => {
          if (prev.length === 0) return [newCandle]
          const last = prev[prev.length - 1]
          
          if (newCandle.time > last.time) {
            return [...prev, newCandle]
          } else if (newCandle.time === last.time) {
            const newArr = [...prev]
            newArr[newArr.length - 1] = newCandle
            return newArr
          }
          return prev
        })
      }
    }

    ws.onerror = (err) => console.error('HL WS Error:', err)
    
    return () => {
      if (ws.readyState === WebSocket.OPEN) ws.close()
    }
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
