'use client'

import { create } from 'zustand'
import { immer } from 'zustand/middleware/immer'

// ─── Types (mirror engine WS types) ──────────────────────────────────────────

export interface PriceLevel {
  price: number
  total_qty: number
  order_count: number
}

export interface OrderbookSnapshot {
  market_id: string
  bids: PriceLevel[]
  asks: PriceLevel[]
  timestamp: number
}

export interface MarketTicker {
  market_id: string
  price: number
  price_24h_ago: number
  change_24h: number
  change_pct_24h: number
  volume_24h: number
  high_24h: number
  low_24h: number
  open_interest: number
  funding_rate: number
  next_funding_ts: number
  timestamp: number
}

export interface Trade {
  price: number
  size: number
  side: 'buy' | 'sell'
  timestamp: number
}

export interface Position {
  market_id: string
  owner: string
  side: 'long' | 'short'
  size: number
  entry_price: number
  mark_price: number
  collateral: number
  leverage: number
  unrealized_pnl: number
  liquidation_price: number
  funding_payment: number
  opened_at: number
}

export interface PolyOpportunity {
  market_id: string
  question: string
  category: string
  end_date: string
  yes_price: number
  no_price: number
  volume_24h: number
  liquidity: number
  best_bid: number
  best_ask: number
  spread: number
  source: string
}

export interface OpenOrder {
  id: number
  market_id: string
  side: 'bid' | 'ask'
  price: number
  quantity: number
  remaining: number
  status: string
  timestamp: number
}

// ─── Store ────────────────────────────────────────────────────────────────────

interface MarketStore {
  activeMarket: string
  orderbook: OrderbookSnapshot | null
  ticker: MarketTicker | null
  tickers: Record<string, MarketTicker>
  recentTrades: Trade[]
  positions: Position[]
  openOrders: OpenOrder[]
  polyOpportunities: PolyOpportunity[]
  pendingTxs: { requestId: string; description: string; txBase64: string }[]

  setActiveMarket: (id: string) => void
  handleEngineMessage: (msg: unknown) => void
  addPosition: (p: Position) => void
  removePosition: (marketId: string) => void
}

export const useMarketStore = create<MarketStore>()(
  immer((set) => ({
    activeMarket: 'SOL-USDC',
    orderbook: null,
    ticker: null,
    tickers: {},
    recentTrades: [],
    positions: [],
    openOrders: [],
    polyOpportunities: [],
    pendingTxs: [],

    setActiveMarket: (id) => set((s) => { s.activeMarket = id }),

    handleEngineMessage: (msg: any) => {
      if (!msg?.type) return

      set((s) => {
        switch (msg.type) {
          case 'snapshot':
            if (msg.market_id === s.activeMarket || !msg.market_id) {
              s.orderbook = msg as OrderbookSnapshot
            }
            break

          case 'ticker_update': {
            const t: MarketTicker = msg.ticker
            s.tickers[t.market_id] = t
            if (t.market_id === s.activeMarket) {
              s.ticker = t
            }
            break
          }

          case 'fill_update': {
            const fill = msg.fill
            s.recentTrades.unshift({
              price: parseFloat(fill.price),
              size: parseFloat(fill.quantity),
              side: fill.maker_side === 'Ask' ? 'sell' : 'buy',
              timestamp: fill.timestamp,
            })
            if (s.recentTrades.length > 100) s.recentTrades.length = 100
            break
          }

          case 'polymarket_opportunities':
            s.polyOpportunities = msg.opportunities
            break

          case 'order_ack':
            // Could add to openOrders here
            break

          case 'unsigned_tx':
            s.pendingTxs.push({
              requestId: msg.request_id,
              description: msg.description,
              txBase64: msg.tx_base64,
            })
            break

          case 'error':
            console.error('[Engine]', msg.code, msg.message)
            break
        }
      })
    },

    addPosition: (p) => set((s) => { s.positions.push(p) }),
    removePosition: (marketId) => set((s) => {
      s.positions = s.positions.filter(p => p.market_id !== marketId)
    }),
  }))
)
