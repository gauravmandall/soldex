"use client";

// ── MarketSelector ────────────────────────────────────────────────────────────

import { useEngine } from "@/providers/EngineProvider";
import type { MarketId } from "./TradingTerminal";
import { useEffect, useRef } from "react";
import { createChart, IChartApi, ISeriesApi, CandlestickData } from "lightweight-charts";
import { formatPrice, formatVolume } from "@/lib/format";

const MARKETS: { id: MarketId; label: string }[] = [
  { id: "SOL-PERP", label: "SOL-PERP" },
  { id: "BTC-PERP", label: "BTC-PERP" },
  { id: "ETH-PERP", label: "ETH-PERP" },
];

export function MarketSelector({
  active,
  onChange,
}: {
  active: MarketId;
  onChange: (id: MarketId) => void;
}) {
  const { tickers } = useEngine();

  return (
    <div className="flex items-center h-10 px-4 border-b border-[var(--bg-border)] bg-[var(--bg-panel)] gap-1 shrink-0">
      {MARKETS.map(({ id, label }) => {
        const t = tickers[id];
        const isUp = (t?.change_pct_24h ?? 0) >= 0;
        const isActive = id === active;

        return (
          <button
            key={id}
            onClick={() => onChange(id)}
            className={[
              "flex items-center gap-3 px-3 py-1.5 rounded text-xs transition-all",
              isActive
                ? "bg-[var(--bg-elevated)] text-[var(--text-primary)]"
                : "text-[var(--text-muted)] hover:text-[var(--text-secondary)]",
            ].join(" ")}
          >
            <span className="font-mono font-medium">{label}</span>
            {t && (
              <>
                <span className={`font-mono tabular ${isUp ? "price-up" : "price-down"}`}>
                  {formatPrice(t.price)}
                </span>
                <span className={`font-mono ${isUp ? "price-up" : "price-down"}`}>
                  {isUp ? "+" : ""}{t.change_pct_24h.toFixed(2)}%
                </span>
              </>
            )}
          </button>
        );
      })}

      {/* Extended stats for active market */}
      {tickers[active] && (
        <div className="flex items-center gap-6 ml-4 text-xs">
          {[
            { label: "24h Vol", value: formatVolume(tickers[active].volume_24h) },
            { label: "24h High", value: formatPrice(tickers[active].high_24h) },
            { label: "24h Low", value: formatPrice(tickers[active].low_24h) },
            { label: "OI", value: formatVolume(tickers[active].open_interest) },
            {
              label: "Funding",
              value: `${(tickers[active].funding_rate * 100).toFixed(4)}%`,
              color: tickers[active].funding_rate >= 0 ? "var(--green)" : "var(--red)",
            },
          ].map(({ label, value, color }) => (
            <div key={label} className="flex items-center gap-1.5">
              <span className="text-[var(--text-muted)]">{label}</span>
              <span
                className="font-mono tabular"
                style={{ color: color ?? "var(--text-secondary)" }}
              >
                {value}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// ── ChartPanel ────────────────────────────────────────────────────────────────

const CHART_THEME = {
  background: { color: "#0d1117" },
  textColor: "#7d8590",
  grid: {
    vertLines: { color: "#1e2d3d" },
    horzLines: { color: "#1e2d3d" },
  },
  crosshair: {
    vertLine: { color: "#3b82f6", labelBackgroundColor: "#141c24" },
    horzLine: { color: "#3b82f6", labelBackgroundColor: "#141c24" },
  },
  rightPriceScale: {
    borderColor: "#1e2d3d",
  },
  timeScale: {
    borderColor: "#1e2d3d",
  },
};

// Generate fake OHLCV data until we have a real feed
function generateCandles(basePrice: number, count = 200): CandlestickData[] {
  const candles: CandlestickData[] = [];
  let price = basePrice;
  const now = Math.floor(Date.now() / 1000);

  for (let i = count; i >= 0; i--) {
    const change = (Math.random() - 0.5) * 0.02 * price;
    const open = price;
    const close = price + change;
    const high = Math.max(open, close) * (1 + Math.random() * 0.005);
    const low = Math.min(open, close) * (1 - Math.random() * 0.005);
    const volume = Math.random() * 1000;

    candles.push({
      time: (now - i * 300) as any, // 5-min bars
      open,
      high,
      low,
      close,
      value: volume,
    } as any);

    price = close;
  }
  return candles;
}

const MARKET_BASE_PRICES: Record<string, number> = {
  "SOL-PERP": 150,
  "BTC-PERP": 65000,
  "ETH-PERP": 3400,
};

export function ChartPanel({ marketId }: { marketId: MarketId }) {
  const containerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const candleSeriesRef = useRef<ISeriesApi<"Candlestick"> | null>(null);
  const { tickers } = useEngine();

  useEffect(() => {
    if (!containerRef.current) return;

    const chart = createChart(containerRef.current, {
      ...CHART_THEME,
      width: containerRef.current.clientWidth,
      height: containerRef.current.clientHeight,
      layout: CHART_THEME,
      handleScroll: true,
      handleScale: true,
    });

    chartRef.current = chart;

    const candleSeries = chart.addCandlestickSeries({
      upColor: "#00d4a0",
      downColor: "#ff4a6b",
      borderUpColor: "#00d4a0",
      borderDownColor: "#ff4a6b",
      wickUpColor: "#00d4a0",
      wickDownColor: "#ff4a6b",
    });

    candleSeriesRef.current = candleSeries;
    candleSeries.setData(generateCandles(MARKET_BASE_PRICES[marketId] ?? 100));
    chart.timeScale().fitContent();

    const ro = new ResizeObserver(() => {
      if (containerRef.current) {
        chart.applyOptions({
          width: containerRef.current.clientWidth,
          height: containerRef.current.clientHeight,
        });
      }
    });
    ro.observe(containerRef.current);

    return () => {
      ro.disconnect();
      chart.remove();
    };
  }, [marketId]);

  // Update last candle's close with live price
  useEffect(() => {
    const ticker = tickers[marketId];
    if (!ticker || !candleSeriesRef.current) return;

    // lightweight-charts: update last bar's close
    // In production, subscribe to real OHLCV from engine
  }, [tickers, marketId]);

  return (
    <div className="flex flex-col h-full">
      {/* Timeframe selector */}
      <div className="flex items-center gap-1 px-3 py-1.5 border-b border-[var(--bg-border)]">
        {["1m", "5m", "15m", "1H", "4H", "1D"].map((tf) => (
          <button
            key={tf}
            className={`px-2 py-0.5 text-xs font-mono rounded transition-colors ${
              tf === "5m"
                ? "bg-[var(--bg-elevated)] text-[var(--text-primary)]"
                : "text-[var(--text-muted)] hover:text-[var(--text-secondary)]"
            }`}
          >
            {tf}
          </button>
        ))}
      </div>

      {/* Chart */}
      <div ref={containerRef} className="flex-1 min-h-0" />
    </div>
  );
}
