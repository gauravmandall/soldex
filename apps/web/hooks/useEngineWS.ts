'use client'

/**
 * useEngineWS
 * ─────────────────────────────────────────────────────────────────────────────
 * Persistent WebSocket connection to the Soldex Rust engine.
 * Handles reconnection with exponential backoff, message routing,
 * and heartbeat ping/pong.
 */

import { useEffect, useRef, useCallback } from 'react'
import { create } from 'zustand'
import { useMarketStore } from './useMarketStore'

const ENGINE_URL = process.env.NEXT_PUBLIC_ENGINE_WS_URL ?? 'ws://localhost:9000/ws'
const PING_INTERVAL_MS = 20_000
const MAX_BACKOFF_MS = 30_000

type ConnectionStatus = 'connecting' | 'connected' | 'disconnected'

interface EngineWSState {
  status: ConnectionStatus
  connected: boolean
  lastPong: number
  _ws: WebSocket | null
  _setWS: (ws: WebSocket | null) => void
  _setStatus: (s: ConnectionStatus) => void
}

const useWSStore = create<EngineWSState>((set) => ({
  status: 'disconnected',
  connected: false,
  lastPong: 0,
  _ws: null,
  _setWS: (ws) => set({ _ws: ws }),
  _setStatus: (status) => set({ status, connected: status === 'connected' }),
}))

export function useEngineWS() {
  const { _ws, connected, status, _setWS, _setStatus } = useWSStore()
  const { handleEngineMessage } = useMarketStore()

  const reconnectTimeout = useRef<NodeJS.Timeout | null>(null)
  const pingInterval     = useRef<NodeJS.Timeout | null>(null)
  const backoff          = useRef(1000)
  const unmounted        = useRef(false)

  const connect = useCallback(() => {
    if (unmounted.current) return

    _setStatus('connecting')
    let ws: WebSocket

    try {
      ws = new WebSocket(ENGINE_URL)
    } catch {
      scheduleReconnect()
      return
    }

    ws.onopen = () => {
      if (unmounted.current) { ws.close(); return }
      _setWS(ws)
      _setStatus('connected')
      backoff.current = 1000 // reset on success

      // Subscribe to active market
      const market = useMarketStore.getState().activeMarket
      ws.send(JSON.stringify({ type: 'subscribe', market_id: market }))
      ws.send(JSON.stringify({ type: 'get_poly_opportunities' }))

      // Heartbeat
      pingInterval.current = setInterval(() => {
        if (ws.readyState === WebSocket.OPEN) {
          ws.send(JSON.stringify({ type: 'ping' }))
        }
      }, PING_INTERVAL_MS)
    }

    ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data as string)
        handleEngineMessage(msg)
        if (msg.type === 'pong') {
          useWSStore.setState({ lastPong: Date.now() })
        }
      } catch { /* ignore parse errors */ }
    }

    ws.onerror = () => { /* handled in onclose */ }

    ws.onclose = () => {
      clearInterval(pingInterval.current!)
      _setWS(null)
      _setStatus('disconnected')
      if (!unmounted.current) scheduleReconnect()
    }

    _setWS(ws)
  }, [_setWS, _setStatus, handleEngineMessage])

  const scheduleReconnect = useCallback(() => {
    reconnectTimeout.current = setTimeout(() => {
      backoff.current = Math.min(backoff.current * 2, MAX_BACKOFF_MS)
      connect()
    }, backoff.current)
  }, [connect])

  useEffect(() => {
    unmounted.current = false
    connect()
    return () => {
      unmounted.current = true
      clearTimeout(reconnectTimeout.current!)
      clearInterval(pingInterval.current!)
      useWSStore.getState()._ws?.close()
    }
  }, []) // only on mount

  // Re-subscribe when active market changes
  const sendSubscribe = useCallback((marketId: string) => {
    const ws = useWSStore.getState()._ws
    if (ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ type: 'subscribe', market_id: marketId }))
    }
  }, [])

  const sendMessage = useCallback((msg: object) => {
    const ws = useWSStore.getState()._ws
    if (ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(msg))
    }
  }, [])

  return { connected, status, sendMessage, sendSubscribe }
}
