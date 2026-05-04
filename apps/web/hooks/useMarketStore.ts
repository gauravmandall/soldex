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
  yes_token_id: string
  no_token_id: string
  volume_24h: number
  liquidity: number
  best_bid: number
  best_ask: number
  spread: number
  source: string
}

export interface JupiterOpportunity {
  event_id: string
  market_id: string
  title: string
  image_url: string | null
  outcomes: string[]
  outcome_prices: number[]
  clob_token_ids: string[]
  volume_24h: number
  volume_usd: number
  category: string
  is_live: boolean
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

export interface PolyOrderbook {
  market_id: string
  asset_id: string
  bids: { price: number; size: number }[]
  asks: { price: number; size: number }[]
  timestamp: string
}

export interface PolyTrade {
  asset_id: string
  price: number
  size: number
  side: 'BUY' | 'SELL'
  timestamp: string
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
  selectedPolyMarketId: string | null
  jupiterOpportunities: JupiterOpportunity[]
  selectedJupiterMarketId: string | null
  polyOrderbooks: Record<string, PolyOrderbook> // key: asset_id or market_id
  polyTrades: Record<string, PolyTrade[]>
  polyMarketHistory: { time: number; price: number }[]
  pendingTxs: { requestId: string; description: string; txBase64: string }[]

  setActiveMarket: (id: string) => void
  setSelectedPolyMarket: (id: string | null) => void
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
    selectedPolyMarketId: null,
    jupiterOpportunities: [],
    selectedJupiterMarketId: null,
    polyOrderbooks: {},
    polyTrades: {},
    polyMarketHistory: [],
    pendingTxs: [],

    setActiveMarket: (id) => set((s) => { s.activeMarket = id }),
    setSelectedPolyMarket: (id) => set((s) => { 
      s.selectedPolyMarketId = id;
      if (id) s.selectedJupiterMarketId = null;
    }),
    setSelectedJupiterMarket: (id) => set((s) => { 
      s.selectedJupiterMarketId = id;
      if (id) s.selectedPolyMarketId = null;
    }),

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
            // Merge with existing ticker to preserve stats (vol, high, low) if not provided
            const existing = s.tickers[t.market_id]
            if (existing) {
              s.tickers[t.market_id] = {
                ...existing,
                ...t,
                // Only overwrite stats if they are non-zero in the new update
                volume_24h: t.volume_24h || existing.volume_24h,
                high_24h: t.high_24h || existing.high_24h,
                low_24h: t.low_24h || existing.low_24h,
                change_pct_24h: t.change_pct_24h || existing.change_pct_24h,
              }
            } else {
              s.tickers[t.market_id] = t
            }
            
            if (t.market_id === s.activeMarket) {
              s.ticker = s.tickers[t.market_id]
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

          case 'jupiter_prediction_opportunities':
            s.jupiterOpportunities = msg.opportunities
            break

          case 'polymarket_update': {
            const update = msg.update
            if (update.type === 'book') {
              const book: PolyOrderbook = {
                market_id: update.market_id,
                asset_id: update.asset_id,
                bids: update.bids.map((b: any) => ({ price: parseFloat(b.price), size: parseFloat(b.size) })),
                asks: update.asks.map((a: any) => ({ price: parseFloat(a.price), size: parseFloat(a.size) })),
                timestamp: update.timestamp,
              }
              s.polyOrderbooks[book.asset_id] = book
              
              // Also update the opportunity price if it exists
              const opp = s.polyOpportunities.find(o => o.market_id === book.market_id)
              if (opp) {
                // Determine if this is YES or NO token? 
                // Usually market_id in opportunities is the condition_id.
                // In WS, market_id might be condition_id too.
                // For now, let's assume we can find the opp and update its best bid/ask
                if (book.bids.length > 0) opp.best_bid = book.bids[0].price
                if (book.asks.length > 0) opp.best_ask = book.asks[0].price
                opp.yes_price = (opp.best_bid + opp.best_ask) / 2
              }
            } else if (update.type === 'trades') {
              const trade: PolyTrade = {
                asset_id: update.asset_id,
                price: parseFloat(update.price),
                size: parseFloat(update.size),
                side: update.side,
                timestamp: update.timestamp,
              }
              if (!s.polyTrades[trade.asset_id]) s.polyTrades[trade.asset_id] = []
              s.polyTrades[trade.asset_id].unshift(trade)
              if (s.polyTrades[trade.asset_id].length > 50) s.polyTrades[trade.asset_id].length = 50
            }
            break
          }

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
