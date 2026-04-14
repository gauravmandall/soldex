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
    <div className="relative flex items-center h-[20px] px-2 font-mono text-[11px] tabular hover:bg-[#1c212e] transition-colors cursor-pointer group">
      {/* Depth bar */}
      <div
        className="absolute inset-y-0 pointer-events-none transition-all duration-300"
        style={{
          right: 0,
          width: `${pct}%`,
          background: isAsk
            ? "rgba(239, 68, 68, 0.08)"
            : "rgba(16, 185, 129, 0.08)",
        }}
      />

      {/* Price */}
      <span
        className="flex-1 relative z-10"
        style={{ color: isAsk ? "#f87171" : "#34d399" }}
      >
        {level.price.toFixed(3)}
      </span>

      {/* Size */}
      <span className="w-20 text-right relative z-10 text-[#94a3b8]">
        {level.total_qty.toFixed(2)}
      </span>

      {/* Total */}
      <span className="w-24 text-right relative z-10 text-[#64748b]">
        {level.total.toFixed(2)}
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

    let cumAsk = 0;
    const asks = [...book.asks]
      .slice(0, DEPTH)
      .map((l) => {
        cumAsk += l.total_qty;
        return { ...l, total: cumAsk };
      })
      .reverse();

    let cumBid = 0;
    const bids = book.bids.slice(0, DEPTH).map((l) => {
      cumBid += l.total_qty;
      return { ...l, total: cumBid };
    });

    const maxTotal = Math.max(
      asks.length ? asks[asks.length - 1].total : 0,
      bids.length ? bids[bids.length - 1].total : 0,
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

  const { buyPct, sellPct } = useMemo(() => {
    if (!book) return { buyPct: 50, sellPct: 50 };
    const bidTotal = book.bids.reduce((s, l) => s + l.total_qty, 0);
    const askTotal = book.asks.reduce((s, l) => s + l.total_qty, 0);
    const tot = bidTotal + askTotal || 1;
    return { buyPct: (bidTotal / tot) * 100, sellPct: (askTotal / tot) * 100 };
  }, [book]);

  return (
    <div className="flex flex-col h-full bg-[#0a0e15] text-[#e2e8f0]">
      {/* Column headers */}
      <div className="grid grid-cols-[1fr_80px_96px] px-2 py-2 border-b border-[#1e2634] text-[10px] font-bold text-[#4a5568] uppercase tracking-wider">
        <span>Price</span>
        <span className="text-right">Size</span>
        <span className="text-right">Total</span>
      </div>

      {/* Asks */}
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

      {/* Mid Price */}
      <div className="flex items-center gap-3 px-2 py-2 border-y border-[#1e2634] bg-[#111827]">
        <span className={`font-mono text-[14px] font-bold tabular ${
           (ticker?.price ?? 0) >= 0 ? "text-[#10b981]" : "text-[#ef4444]"
        }`}>
          {ticker ? ticker.price.toFixed(3) : "---"}
        </span>
        <div className="flex flex-col">
          <span className="text-[9px] text-[#4a5568] font-bold uppercase leading-none">Spread</span>
          <span className="text-[10px] font-mono text-[#64748b]">
            {spread ? `${spread.abs.toFixed(3)} (${spread.pct.toFixed(3)}%)` : "---"}
          </span>
        </div>
        <div className="ml-auto flex items-center gap-1">
          <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="#4a5568" strokeWidth="3"><path d="M12 2v20M17 5l-5-5-5 5M17 19l-5 5-5-5"/></svg>
        </div>
      </div>

      {/* Bids */}
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

      {/* Sentiment Bar */}
      <div className="px-2 py-3 border-t border-[#1e2634] bg-[#0d1117]">
        <div className="flex h-1.5 rounded-full overflow-hidden bg-[#1e2634]">
          <div
            className="bg-[#10b981] transition-all duration-700 shadow-[0_0_8px_rgba(16,185,129,0.3)]"
            style={{ width: `${buyPct}%` }}
          />
          <div
            className="bg-[#ef4444] transition-all duration-700 shadow-[0_0_8px_rgba(239,68,68,0.3)]"
            style={{ width: `${sellPct}%` }}
          />
        </div>
        <div className="flex justify-between mt-2 font-bold font-mono text-[10px] tracking-tight">
          <span className="text-[#10b981]">{buyPct.toFixed(1)}% Buy</span>
          <span className="text-[#ef4444]">{sellPct.toFixed(1)}% Sell</span>
        </div>
      </div>
    </div>
  );
}
