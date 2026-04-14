"use client";

import { useMemo } from "react";
import { useEngine, OrderbookLevel } from "@/providers/EngineProvider";
import type { MarketId } from "./TradingTerminal";
import { formatPrice, formatQty } from "@/lib/format";

const DEPTH = 16;

function DepthRow({
  level,
  side,
  maxTotal,
}: {
  level: OrderbookLevel & { total: number };
  side: "bid" | "ask";
  maxTotal: number;
}) {
  const pct = (level.total / maxTotal) * 100;
  const isAsk = side === "ask";

  return (
    <div className="relative flex items-center h-5 px-2 font-mono text-xs tabular hover:bg-[var(--bg-elevated)] transition-colors cursor-pointer group">
      {/* Depth bar */}
      <div
        className="absolute inset-y-0 pointer-events-none"
        style={{
          [isAsk ? "left" : "right"]: 0,
          width: `${pct}%`,
          background: isAsk
            ? "rgba(255, 74, 107, 0.10)"
            : "rgba(0, 212, 160, 0.10)",
        }}
      />

      {/* Price */}
      <span
        className="flex-1 relative z-10"
        style={{ color: isAsk ? "var(--red)" : "var(--green)" }}
      >
        {formatPrice(level.price)}
      </span>

      {/* Size */}
      <span className="w-20 text-right relative z-10 text-[var(--text-secondary)]">
        {formatQty(level.total_qty)}
      </span>

      {/* Total */}
      <span className="w-24 text-right relative z-10 text-[var(--text-muted)]">
        {formatQty(level.total)}
      </span>
    </div>
  );
}

export function OrderBook({ marketId }: { marketId: MarketId }) {
  const { orderbooks, tickers } = useEngine();
  const book = orderbooks[marketId];
  const ticker = tickers[marketId];

  const { asks, bids, maxTotal } = useMemo(() => {
    if (!book) {
      return { asks: [], bids: [], maxTotal: 1 };
    }

    // Cumulative totals — asks from bottom (closest to mid)
    let cumAsk = 0;
    const asks = [...book.asks]
      .slice(0, DEPTH)
      .map((l) => {
        cumAsk += l.total_qty;
        return { ...l, total: cumAsk };
      })
      .reverse(); // show closest to mid at bottom of ask section

    let cumBid = 0;
    const bids = book.bids.slice(0, DEPTH).map((l) => {
      cumBid += l.total_qty;
      return { ...l, total: cumBid };
    });

    const maxTotal = Math.max(
      asks[asks.length - 1]?.total ?? 0,
      bids[bids.length - 1]?.total ?? 0,
      1
    );

    return { asks, bids, maxTotal };
  }, [book]);

  const spread = useMemo(() => {
    if (!book || !book.asks[0] || !book.bids[0]) return null;
    const spreadAbs = book.asks[0].price - book.bids[0].price;
    const spreadPct = (spreadAbs / book.bids[0].price) * 100;
    return { abs: spreadAbs, pct: spreadPct };
  }, [book]);

  // Buy/sell pressure ratio
  const { buyPct, sellPct } = useMemo(() => {
    if (!book) return { buyPct: 50, sellPct: 50 };
    const bidTotal = book.bids.reduce((s, l) => s + l.total_qty, 0);
    const askTotal = book.asks.reduce((s, l) => s + l.total_qty, 0);
    const tot = bidTotal + askTotal || 1;
    return { buyPct: (bidTotal / tot) * 100, sellPct: (askTotal / tot) * 100 };
  }, [book]);

  return (
    <div className="flex flex-col h-full text-[var(--text-primary)]">
      {/* Column headers */}
      <div className="flex items-center px-2 py-1.5 border-b border-[var(--bg-border)] text-[10px] font-mono text-[var(--text-muted)] uppercase tracking-wider">
        <span className="flex-1">Price</span>
        <span className="w-20 text-right">Size</span>
        <span className="w-24 text-right">Total</span>
      </div>

      {/* Asks (sell orders) */}
      <div className="flex-1 flex flex-col justify-end overflow-hidden">
        {asks.map((level) => (
          <DepthRow
            key={level.price}
            level={level}
            side="ask"
            maxTotal={maxTotal}
          />
        ))}
      </div>

      {/* Mid price / spread */}
      <div className="flex items-center justify-between px-2 py-1.5 border-y border-[var(--bg-border)] bg-[var(--bg-elevated)]">
        <span
          className={`font-mono text-sm font-semibold tabular ${
            (ticker?.change_24h ?? 0) >= 0 ? "price-up" : "price-down"
          }`}
        >
          {ticker ? formatPrice(ticker.price) : "—"}
        </span>
        {spread && (
          <span className="font-mono text-[10px] text-[var(--text-muted)]">
            Spread {spread.pct.toFixed(3)}%
          </span>
        )}
      </div>

      {/* Bids (buy orders) */}
      <div className="flex-1 overflow-hidden">
        {bids.map((level) => (
          <DepthRow
            key={level.price}
            level={level}
            side="bid"
            maxTotal={maxTotal}
          />
        ))}
      </div>

      {/* Buy/sell pressure bar */}
      <div className="px-2 py-2 border-t border-[var(--bg-border)]">
        <div className="flex h-1 rounded-full overflow-hidden">
          <div
            className="bg-[var(--green)] transition-all duration-500"
            style={{ width: `${buyPct}%` }}
          />
          <div
            className="bg-[var(--red)] transition-all duration-500"
            style={{ width: `${sellPct}%` }}
          />
        </div>
        <div className="flex justify-between mt-1 font-mono text-[10px]">
          <span className="price-up">B {buyPct.toFixed(1)}%</span>
          <span className="price-down">S {sellPct.toFixed(1)}%</span>
        </div>
      </div>
    </div>
  );
}
