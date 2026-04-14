"use client";

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";

const ENGINE_WS = process.env.NEXT_PUBLIC_ENGINE_WS_URL ?? "ws://localhost:9000/ws";
const RECONNECT_DELAY_MS = 2000;
const MAX_RECONNECT_ATTEMPTS = 10;

// ── Message types (mirror Rust ws/mod.rs) ────────────────────────────────────

export interface OrderbookLevel {
  price: number;
  total_qty: number;
  order_count: number;
}

export interface OrderbookSnapshot {
  market_id: string;
  bids: OrderbookLevel[];
  asks: OrderbookLevel[];
  timestamp: number;
}

export interface MarketTicker {
  market_id: string;
  price: number;
  price_24h_ago: number;
  change_24h: number;
  change_pct_24h: number;
  volume_24h: number;
  high_24h: number;
  low_24h: number;
  open_interest: number;
  funding_rate: number;
  next_funding_ts: number;
  timestamp: number;
}

export interface PolyOpportunity {
  market_id: string;
  question: string;
  category: string;
  end_date: string;
  yes_price: number;
  no_price: number;
  volume_24h: number;
  liquidity: number;
  best_bid: number;
  best_ask: number;
  spread: number;
  source: string;
}

export interface Fill {
  trade_id: number;
  market_id: string;
  maker_order_id: number;
  taker_order_id: number;
  price: number;
  quantity: number;
  maker_side: "Bid" | "Ask";
  timestamp: number;
}

type ServerMessage =
  | { type: "snapshot"; market_id: string; bids: OrderbookLevel[]; asks: OrderbookLevel[]; timestamp: number }
  | { type: "ticker_update"; ticker: MarketTicker }
  | { type: "polymarket_opportunities"; opportunities: PolyOpportunity[] }
  | { type: "fill_update"; fill: Fill }
  | { type: "order_ack"; client_order_id: string; order_id: number; status: string }
  | { type: "unsigned_tx"; request_id: string; tx_base64: string; description: string }
  | { type: "error"; code: string; message: string }
  | { type: "pong" };

// ── Context ───────────────────────────────────────────────────────────────────

interface EngineContextValue {
  connected: boolean;
  tickers: Record<string, MarketTicker>;
  orderbooks: Record<string, OrderbookSnapshot>;
  polyOpportunities: PolyOpportunity[];
  recentFills: Fill[];
  /** Send a raw message to the engine */
  send: (msg: object) => void;
  /** Subscribe to a market's orderbook */
  subscribe: (marketId: string) => void;
  /** Place an order */
  placeOrder: (params: {
    marketId: string;
    side: "bid" | "ask";
    orderType: "market" | "limit";
    price: number;
    quantity: number;
    ownerPubkey: string;
    signedTx?: string;
  }) => string; // returns client_order_id
  /** Request Polymarket opportunities */
  requestPolyOpportunities: () => void;
}

const EngineContext = createContext<EngineContextValue | null>(null);

export function EngineProvider({ children }: { children: React.ReactNode }) {
  const ws = useRef<WebSocket | null>(null);
  const reconnectCount = useRef(0);
  const [connected, setConnected] = useState(false);
  const [tickers, setTickers] = useState<Record<string, MarketTicker>>({});
  const [orderbooks, setOrderbooks] = useState<Record<string, OrderbookSnapshot>>({});
  const [polyOpportunities, setPolyOpportunities] = useState<PolyOpportunity[]>([]);
  const [recentFills, setRecentFills] = useState<Fill[]>([]);

  const send = useCallback((msg: object) => {
    if (ws.current?.readyState === WebSocket.OPEN) {
      ws.current.send(JSON.stringify(msg));
    }
  }, []);

  const handleMessage = useCallback((raw: string) => {
    try {
      const msg: ServerMessage = JSON.parse(raw);
      switch (msg.type) {
        case "snapshot":
          setOrderbooks((prev) => ({
            ...prev,
            [msg.market_id]: {
              market_id: msg.market_id,
              bids: msg.bids,
              asks: msg.asks,
              timestamp: msg.timestamp,
            },
          }));
          break;

        case "ticker_update":
          setTickers((prev) => ({ ...prev, [msg.ticker.market_id]: msg.ticker }));
          break;

        case "polymarket_opportunities":
          setPolyOpportunities(msg.opportunities);
          break;

        case "fill_update":
          setRecentFills((prev) => [msg.fill, ...prev.slice(0, 99)]);
          break;

        case "error":
          console.error(`[Engine] ${msg.code}: ${msg.message}`);
          break;
      }
    } catch (e) {
      console.error("[Engine] Parse error:", e);
    }
  }, []);

  const connect = useCallback(() => {
    if (ws.current) ws.current.close();

    const socket = new WebSocket(ENGINE_WS);
    ws.current = socket;

    socket.onopen = () => {
      setConnected(true);
      reconnectCount.current = 0;
      // Subscribe to all perps markets on connect
      ["SOL-PERP", "BTC-PERP", "ETH-PERP"].forEach((id) => {
        socket.send(JSON.stringify({ type: "subscribe", market_id: id }));
      });
    };

    socket.onmessage = (e) => handleMessage(e.data);

    socket.onclose = () => {
      setConnected(false);
      if (reconnectCount.current < MAX_RECONNECT_ATTEMPTS) {
        reconnectCount.current++;
        setTimeout(connect, RECONNECT_DELAY_MS);
      }
    };

    socket.onerror = () => {
      socket.close();
    };
  }, [handleMessage]);

  useEffect(() => {
    connect();
    return () => ws.current?.close();
  }, [connect]);

  // Heartbeat
  useEffect(() => {
    const id = setInterval(() => {
      if (connected) send({ type: "ping" });
    }, 30_000);
    return () => clearInterval(id);
  }, [connected, send]);

  const subscribe = useCallback(
    (marketId: string) => {
      send({ type: "subscribe", market_id: marketId });
      send({ type: "get_snapshot", market_id: marketId, depth: 20 });
    },
    [send]
  );

  const placeOrder = useCallback(
    (params: {
      marketId: string;
      side: "bid" | "ask";
      orderType: "market" | "limit";
      price: number;
      quantity: number;
      ownerPubkey: string;
      signedTx?: string;
    }) => {
      const clientOrderId = `${Date.now()}-${Math.random().toString(36).slice(2)}`;
      send({
        type: "place_order",
        market_id: params.marketId,
        side: params.side,
        order_type: params.orderType,
        price: params.price,
        quantity: params.quantity,
        owner_pubkey: params.ownerPubkey,
        signed_tx: params.signedTx ?? null,
        client_order_id: clientOrderId,
      });
      return clientOrderId;
    },
    [send]
  );

  const requestPolyOpportunities = useCallback(() => {
    send({ type: "get_poly_opportunities" });
  }, [send]);

  return (
    <EngineContext.Provider
      value={{
        connected,
        tickers,
        orderbooks,
        polyOpportunities,
        recentFills,
        send,
        subscribe,
        placeOrder,
        requestPolyOpportunities,
      }}
    >
      {children}
    </EngineContext.Provider>
  );
}

export function useEngine() {
  const ctx = useContext(EngineContext);
  if (!ctx) throw new Error("useEngine must be inside EngineProvider");
  return ctx;
}
