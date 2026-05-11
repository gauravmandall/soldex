"use client";

import React, { useState, useEffect } from "react";
import { useMarketStore } from "@/hooks/useMarketStore";
import { useEngineWS } from "@/hooks/useEngineWS";
import { TopBar } from "./TopBar";
import { MarketSelector } from "./MarketSelector";
import { CandleChart } from "../charts/CandleChart";
import { OrderBook } from "./OrderBook";
import { TradesPanel } from "./TradesPanel";
import { OrderPanel } from "./OrderPanel";
import { PositionsPanel } from "./PositionsPanel";
import { PolymarketPanel } from "../polymarket/PolymarketPanel";
import { PrivacyBanner } from "../privacy/PrivacyBanner";
import { ErLatencyBar } from "./ErLatencyBar";
import { PendingTxModal } from './PendingTxModal'

import {
  Zap,
  Activity,
  Info,
  ChevronRight,
  BarChart3,
  Clock,
} from "lucide-react";

export const MARKETS = ["SOL-USDC", "BTC-USDC", "ETH-USDC", "JUP-USDC"];
export type MarketId = (typeof MARKETS)[number] | string;
export type RightPanel = "orderbook" | "trades" | "polymarket";
export type BottomPanel = "positions" | "orders" | "history";

interface Props {
  defaultMarket?: string;
}

export function TradingTerminal({ defaultMarket = "SOL-USDC" }: Props) {
  const { ticker, setActiveMarket, activeMarket, polyOpportunities } =
    useMarketStore();
  const { connected } = useEngineWS();
  const [rightPanel, setRightPanel] = useState<RightPanel>("orderbook");
  const [bottomPanel, setBottomPanel] = useState<BottomPanel>("positions");
  const [chartInterval, setChartInterval] = useState<
    "1m" | "5m" | "15m" | "30m" | "1h" | "4h" | "1d"
  >("30m");

  useEffect(() => {
    setActiveMarket(defaultMarket);
  }, [defaultMarket, setActiveMarket]);

  const PAIRS = [
    {
      symbol: "SOL-USDC",
      price: ticker?.market_id === "SOL-USDC" ? ticker.price : 145.83,
      change: 4.72,
    },
    {
      symbol: "BTC-USDC",
      price: ticker?.market_id === "BTC-USDC" ? ticker.price : 64312.5,
      change: -0.23,
    },
    {
      symbol: "ETH-USDC",
      price: ticker?.market_id === "ETH-USDC" ? ticker.price : 3312.4,
      change: -0.65,
    },
    {
      symbol: "JUP-USDC",
      price: ticker?.market_id === "JUP-USDC" ? ticker.price : 1.12,
      change: 2.45,
    },
  ];

  const pxColor = ticker
    ? ticker.change_pct_24h >= 0
      ? "text-[var(--green)]"
      : "text-[var(--red)]"
    : "text-[var(--tx2)]";

  return (
    <div className="flex flex-col h-screen w-screen bg-[var(--bg)] overflow-hidden text-[var(--tx)] font-mono selection:bg-blue-500/30">
     <PendingTxModal />
      <TopBar connected={connected} />
      <PrivacyBanner />
      <ErLatencyBar />

      {/* PAIRS TICKER STRIP */}
      <div className="flex items-center h-[34px] border-b border-[var(--bd)] bg-[var(--bg1)] overflow-x-auto no-scrollbar shrink-0">
        {PAIRS.map((p) => (
          <div
            key={p.symbol}
            onClick={() => setActiveMarket(p.symbol)}
            className={`flex items-center gap-3 px-4 h-full cursor-pointer border-r border-[var(--bd)] transition-all whitespace-nowrap ${
              activeMarket === p.symbol
                ? "bg-[var(--bg2)] border-b-2 border-b-[var(--blue)]"
                : "hover:bg-[var(--bg2)]/50"
            }`}
          >
            <span className="text-[10px] font-bold text-[var(--tx3)] uppercase tracking-tighter">
              {p.symbol}
            </span>
            <span className="text-[11px] font-bold">
              {p.price.toLocaleString(undefined, {
                minimumFractionDigits: p.price < 10 ? 3 : 1,
              })}
            </span>
            <span
              className={`text-[9px] font-bold ${p.change >= 0 ? "text-[var(--green)]" : "text-[var(--red)]"}`}
            >
              {p.change >= 0 ? "+" : ""}
              {p.change}%
            </span>
          </div>
        ))}
        {/* POLYMARKET INDICATOR */}
        <div className="flex items-center h-full px-4 gap-4 overflow-hidden bg-[var(--blue)]/5">
          {polyOpportunities.slice(0, 1).map((opp, i) => (
            <div
              key={i}
              className="flex items-center gap-2 group cursor-pointer"
              onClick={() => setRightPanel("polymarket")}
            >
              <div className="text-[8px] font-bold px-1.5 py-0.5 bg-[var(--blue)] text-white rounded-[2px] uppercase tracking-widest">
                Opportunity
              </div>
              <span className="text-[10px] text-[var(--tx2)] group-hover:text-[var(--tx)] transition-colors truncate max-w-[300px] uppercase">
                {opp.question}
              </span>
              <span className="text-[10px] font-bold text-[var(--blue)]">
                {(opp.yes_price * 100).toFixed(0)}¢ YES
              </span>
            </div>
          ))}
        </div>
      </div>

      {/* MAIN MARKET HEADER */}
      <div className="flex items-center h-[56px] px-6 border-b border-[var(--bd)] bg-[var(--bg)] gap-8 shrink-0">
        <div className="flex items-center gap-4">
          <div className="w-9 h-9 border border-[var(--bd)] bg-[var(--bg2)] flex items-center justify-center text-[11px] font-bold text-[var(--tx)] uppercase tracking-widest shadow-inner">
            {activeMarket?.split("-")[0]}
          </div>
          <div className="flex flex-col gap-0.5">
            <div className="flex items-center gap-2">
              <MarketSelector />
              <span className="text-[9px] px-1.5 py-0.5 bg-[var(--blue)]/10 text-[var(--blue)] border border-[var(--blue)]/20 rounded-[2px] font-bold uppercase tracking-widest">
                20x
              </span>
            </div>
            <div className="flex items-center gap-2">
              <span
                className={`text-[18px] font-bold num leading-none ${pxColor}`}
              >
                {ticker?.price.toLocaleString(undefined, {
                  minimumFractionDigits: 3,
                }) || "---"}
              </span>
              <span className={`text-[10px] font-bold num ${pxColor}`}>
                {ticker ? (ticker.change_pct_24h >= 0 ? "+" : "") : ""}
                {ticker?.change_pct_24h.toFixed(2)}%
              </span>
            </div>
          </div>
        </div>

        <div className="h-8 w-[1px] bg-[var(--bd)] mx-2" />

        <div className="flex items-center gap-10 overflow-hidden">
          <Stat
            label="Index Price"
            value={`$${ticker?.price?.toLocaleString(undefined, { minimumFractionDigits: 3 }) || "---"}`}
            vClass="text-[var(--blue)]"
          />
          <Stat
            label="24h High"
            value={`$${ticker?.high_24h?.toLocaleString(undefined, { minimumFractionDigits: 2 }) || "---"}`}
          />
          <Stat
            label="24h Low"
            value={`$${ticker?.low_24h?.toLocaleString(undefined, { minimumFractionDigits: 2 }) || "---"}`}
          />
          <Stat
            label="24h Volume"
            value={`$${ticker ? (ticker.volume_24h / 1_000_000).toFixed(2) : "---"}M`}
          />
          <div className="flex flex-col gap-1">
            <span className="text-[9px] text-[var(--tx3)] font-bold uppercase tracking-widest leading-none">
              Funding / Countdown
            </span>
            <div className="flex items-baseline gap-2">
              <span className="text-[11px] font-bold text-[var(--green)]">
                {ticker ? `${(ticker.funding_rate * 100).toFixed(4)}%` : "---"}
              </span>
              <span className="text-[10px] text-[var(--tx2)]">00:36:39</span>
            </div>
          </div>
        </div>
      </div>

      <div className="flex flex-1 min-h-0 bg-[var(--bg)]">
        {/* CHART AREA */}
        <div className="flex flex-col flex-1 min-w-0 border-r border-[var(--bd)]">
          <div className="flex items-center justify-between px-4 h-[38px] border-b border-[var(--bd)] bg-[var(--bg1)] shrink-0">
            <div className="flex items-center gap-1">
              {(["1m", "5m", "15m", "30m", "1h", "4h", "1d"] as const).map(
                (i) => (
                  <button
                    key={i}
                    onClick={() => setChartInterval(i)}
                    className={`px-3 py-1 text-[10px] font-bold uppercase transition-all ${
                      chartInterval === i
                        ? "bg-[var(--bg2)] text-[var(--blue)] border border-[var(--bd)] shadow-inner"
                        : "text-[var(--tx3)] hover:text-[var(--tx2)]"
                    }`}
                  >
                    {i}
                  </button>
                ),
              )}
            </div>
            <div className="flex items-center gap-4">
              <button className="text-[var(--tx3)] hover:text-[var(--tx)] transition-colors">
                <Zap className="w-4 h-4" />
              </button>
              <button className="text-[var(--tx3)] hover:text-[var(--tx)] transition-colors">
                <Activity className="w-4 h-4" />
              </button>
            </div>
          </div>
          <div className="flex-1 min-h-0 bg-[var(--bg)] p-2">
            <CandleChart marketId={activeMarket} interval={chartInterval} />
          </div>

          {/* POSITIONS AREA */}
          <div className="h-[260px] border-t border-[var(--bd)] flex flex-col shrink-0 bg-[var(--bg1)]">
            <div className="flex border-b border-[var(--bd)] shrink-0 bg-[var(--bg2)]/50">
              {(
                [
                  ["positions", "Positions"],
                  ["orders", "Open Orders"],
                  ["history", "History"],
                ] as const
              ).map(([tab, label]) => (
                <button
                  key={tab}
                  onClick={() => setBottomPanel(tab)}
                  className={`px-6 py-2.5 text-[10px] font-bold uppercase tracking-widest transition-all border-b-2 ${
                    bottomPanel === tab
                      ? "border-[var(--blue)] text-[var(--tx)] bg-[var(--bg2)]"
                      : "border-transparent text-[var(--tx3)] hover:text-[var(--tx2)]"
                  }`}
                >
                  {label}
                </button>
              ))}
            </div>
            <div className="flex-1 overflow-auto no-scrollbar">
              <PositionsPanel view={bottomPanel} />
            </div>
          </div>
        </div>

        {/* RIGHT PANELS */}
        <div className="flex w-[560px] shrink-0">
          {/* BOOK / TRADES */}
          <div className="w-[300px] border-r border-[var(--bd)] flex flex-col bg-[var(--bg1)]">
            <div className="flex border-b border-[var(--bd)] shrink-0 bg-[var(--bg2)]/50">
              {(
                [
                  ["orderbook", "Order Book"],
                  ["trades", "Recent Trades"],
                ] as const
              ).map(([k, l]) => (
                <button
                  key={k}
                  onClick={() => setRightPanel(k as RightPanel)}
                  className={`flex-1 py-3 px-2 text-[10px] font-bold uppercase tracking-widest border-b-2 transition-all whitespace-nowrap ${
                    rightPanel === k
                      ? "border-[var(--blue)] text-[var(--tx)] bg-[var(--bg2)]"
                      : "border-transparent text-[var(--tx3)] hover:text-[var(--tx2)]"
                  }`}
                >
                  {l}
                </button>
              ))}
            </div>
            <div className="flex-1 overflow-hidden">
              {rightPanel === "orderbook" && (
                <OrderBook marketId={activeMarket} />
              )}
              {rightPanel === "trades" && (
                <TradesPanel marketId={activeMarket} />
              )}
              {/* {rightPanel === 'polymarket' && (
                <div className="h-full overflow-y-auto no-scrollbar">
                  <PolymarketPanel hideHeader />
                </div>
              )} */}
            </div>
          </div>

          {/* ORDER PANEL */}
          <div className="w-[260px] bg-[var(--bg)]">
            <OrderPanel marketId={activeMarket} />
          </div>
        </div>
      </div>
    </div>
  );
}

function Stat({
  label,
  value,
  vClass = "text-[var(--tx)]",
}: {
  label: string;
  value: string;
  vClass?: string;
}) {
  return (
    <div className="flex flex-col gap-1 whitespace-nowrap">
      <span className="text-[9px] text-[var(--tx3)] font-bold uppercase tracking-widest leading-none">
        {label}
      </span>
      <span className={`text-[12px] font-bold tracking-tight ${vClass}`}>
        {value}
      </span>
    </div>
  );
}
