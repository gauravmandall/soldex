"use client";

// ── TradeHistory ──────────────────────────────────────────────────────────────

import { useEngine, Fill } from "@/providers/EngineProvider";
import { useSelfCustody } from "@/providers/SelfCustodyProvider";
import type { MarketId } from "./TradingTerminal";
import { formatPrice, formatQty, formatTime } from "@/lib/format";
import { Shield, RefreshCw, AlertTriangle } from "lucide-react";
import { useState } from "react";

export function TradeHistory({ marketId }: { marketId: MarketId }) {
  const { recentFills } = useEngine();
  const fills = recentFills.filter((f) => f.market_id === marketId);

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center px-2 py-1.5 text-[10px] font-mono text-[var(--text-muted)] uppercase tracking-wider border-b border-[var(--bg-border)]">
        <span className="flex-1">Price</span>
        <span className="w-20 text-right">Size</span>
        <span className="w-20 text-right">Time</span>
      </div>

      <div className="flex-1 overflow-y-auto">
        {fills.length === 0 ? (
          <div className="flex items-center justify-center h-20 text-xs text-[var(--text-muted)] font-mono">
            No trades yet
          </div>
        ) : (
          fills.map((fill) => (
            <div
              key={fill.trade_id}
              className="flex items-center px-2 py-1 font-mono text-xs hover:bg-[var(--bg-elevated)] animate-fade-in"
            >
              <span
                className={`flex-1 tabular ${
                  fill.maker_side === "Ask" ? "price-up" : "price-down"
                }`}
              >
                {formatPrice(fill.price)}
              </span>
              <span className="w-20 text-right text-[var(--text-secondary)] tabular">
                {formatQty(fill.quantity)}
              </span>
              <span className="w-20 text-right text-[var(--text-muted)]">
                {formatTime(fill.timestamp)}
              </span>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

// ── PositionsBar ──────────────────────────────────────────────────────────────

type PositionTab = "positions" | "orders" | "history";

export function PositionsBar() {
  const [tab, setTab] = useState<PositionTab>("positions");

  return (
    <div className="h-36 border-t border-[var(--bg-border)] bg-[var(--bg-panel)] flex flex-col shrink-0">
      {/* Tabs */}
      <div className="flex items-center border-b border-[var(--bg-border)] px-2">
        {(["positions", "orders", "history"] as PositionTab[]).map((t) => (
          <button
            key={t}
            onClick={() => setTab(t)}
            className={[
              "px-3 py-2 text-xs font-mono capitalize transition-colors",
              tab === t
                ? "text-[var(--text-primary)] border-b-2 border-[var(--green)] -mb-px"
                : "text-[var(--text-muted)] hover:text-[var(--text-secondary)]",
            ].join(" ")}
          >
            {t}
          </button>
        ))}
      </div>

      {/* Empty state */}
      <div className="flex-1 flex items-center justify-center text-xs text-[var(--text-muted)] font-mono">
        {tab === "positions" && "No open positions"}
        {tab === "orders" && "No open orders"}
        {tab === "history" && "No trade history"}
      </div>
    </div>
  );
}

// ── PrivacyStatusBar ──────────────────────────────────────────────────────────

export function PrivacyStatusBar() {
  const { sessionPubkey, rotationDue, daysUntilRotation, walletHistory, rotateWallet } =
    useSelfCustody();

  if (!sessionPubkey) return null;

  return (
    <div
      className={[
        "flex items-center justify-between px-4 py-1.5 text-xs font-mono border-t shrink-0",
        rotationDue
          ? "border-[var(--red)] bg-[rgba(255,74,107,0.06)] text-[var(--red)]"
          : "border-[var(--bg-border)] bg-[var(--bg-panel)] text-[var(--text-muted)]",
      ].join(" ")}
    >
      <div className="flex items-center gap-2">
        <Shield size={11} className={rotationDue ? "text-[var(--red)]" : "text-[var(--green)]"} />
        <span>
          Session wallet{" "}
          <span className="text-[var(--text-secondary)]">
            {sessionPubkey.slice(0, 6)}…{sessionPubkey.slice(-6)}
          </span>
        </span>
        <span className="text-[var(--text-muted)]">·</span>
        <span>
          {rotationDue ? (
            <span className="flex items-center gap-1 text-[var(--red)]">
              <AlertTriangle size={10} />
              Rotation overdue — rotate now for privacy
            </span>
          ) : (
            `Rotates in ${daysUntilRotation} days`
          )}
        </span>
        {walletHistory.length > 0 && (
          <>
            <span className="text-[var(--text-muted)]">·</span>
            <span>{walletHistory.length} past wallets</span>
          </>
        )}
      </div>

      {rotationDue && (
        <button
          onClick={rotateWallet}
          className="flex items-center gap-1.5 px-3 py-1 rounded border border-[var(--red)] text-[var(--red)] hover:bg-[rgba(255,74,107,0.1)] transition-colors"
        >
          <RefreshCw size={10} />
          Rotate Wallet
        </button>
      )}
    </div>
  );
}
