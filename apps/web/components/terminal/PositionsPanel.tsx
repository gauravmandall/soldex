"use client";
import React from "react";
import { useMarketStore } from "@/hooks/useMarketStore";
import { useSelfCustodyWallet } from "@/hooks/useSelfCustodyWallet";

export function PositionsPanel({
  view,
}: {
  view: "positions" | "orders" | "history";
}) {
  const { positions, openOrders } = useMarketStore();
  const { wallet } = useSelfCustodyWallet();

  if (!wallet)
    return (
      <div className="flex items-center justify-center h-full text-[#4a5568] text-[11px] font-bold uppercase tracking-widest">
        Connect wallet to view status
      </div>
    );

  if (view === "positions")
    return (
      <table className="w-full text-[11px] font-mono border-collapse">
        <thead>
          <tr className="text-[10px] text-[#4a5568] font-bold uppercase tracking-wider border-b border-[#1e2634] bg-[#0d1117]">
            <Th>Market</Th>
            <Th>Side</Th>
            <Th>Size</Th>
            <Th>Entry</Th>
            <Th>Mark</Th>
            <Th>PnL</Th>
            <Th>Liq.</Th>
            <Th>Privacy</Th>{" "}
          </tr>
        </thead>
        <tbody className="bg-[#0a0e15]">
          {positions.map((p, i) => (
            <tr
              key={i}
              className="border-b border-white/[0.03] hover:bg-[#111827] transition-colors group"
            >
              <Td className="font-bold text-[#e2e8f0]">{p.market_id}</Td>
              <Td>
                <span
                  className={`px-1.5 py-0.5 rounded-[3px] font-bold ${
                    p.side === "long"
                      ? "bg-[#064e3b] text-[#34d399]"
                      : "bg-[#4c0519] text-[#fb7185]"
                  }`}
                >
                  {p.side.toUpperCase()}
                </span>
              </Td>
              <Td className="num text-[#94a3b8]">{p.size}</Td>
              <Td className="num text-[#94a3b8]">
                $
                {p.entry_price.toLocaleString(undefined, {
                  minimumFractionDigits: 2,
                })}
              </Td>
              <Td className="num text-[#94a3b8]">
                {" "}
                $
                {p.mark_price.toLocaleString(undefined, {
                  minimumFractionDigits: 2,
                })}
              </Td>
              <Td
                className={`num font-bold ${p.unrealized_pnl >= 0 ? "text-[#10b981]" : "text-[#ef4444]"}`}
              >
                {p.unrealized_pnl >= 0 ? "▲" : "▼"} $
                {Math.abs(p.unrealized_pnl).toFixed(2)}
              </Td>
              <Td className="num text-[#f59e0b] font-semibold">
                ${p.liquidation_price.toFixed(2)}
              </Td>
              <Td>
                {p.delegation_status === "TEE_ENCRYPTED" && (
                  <span
                    title="Mark price & PnL are encrypted on-chain. MEV bots cannot read this position."
                    className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded-[3px] border border-[#3b82f6]/30 text-[#3b82f6] text-[9px] font-bold uppercase tracking-wider cursor-help"
                  >
                    <span className="w-1.5 h-1.5 rounded-full bg-[#3b82f6] animate-pulse inline-block" />
                    TEE
                  </span>
                )}
              </Td>{" "}
            </tr>
          ))}
          {positions.length === 0 && (
            <tr>
              <td colSpan={8} className="text-center py-12">
                {" "}
                <TeePrivacyDemo />
              </td>
            </tr>
          )}
        </tbody>
      </table>
    );

  return (
    <div className="flex flex-col items-center justify-center h-full gap-2 opacity-40">
      <span className="text-[#4a5568] font-bold uppercase tracking-widest text-[10px]">
        No {view} found
      </span>
    </div>
  );
}

const Th = ({ children }: { children: React.ReactNode }) => (
  <th className="px-4 py-2.5 text-left font-bold">{children}</th>
);
const Td = ({
  children,
  className = "",
}: {
  children: React.ReactNode;
  className?: string;
}) => <td className={`px-4 py-3 ${className}`}>{children}</td>;

function TeePrivacyDemo() {
  const [state, setState] = React.useState<
    "base" | "delegating" | "tee" | "settling"
  >("base");
  const [mark, setMark] = React.useState(93.62);

  React.useEffect(() => {
    const id = setInterval(
      () => setMark((p) => +(p + (Math.random() - 0.47) * 0.12).toFixed(3)),
      600,
    );
    return () => clearInterval(id);
  }, []);

  React.useEffect(() => {
    const cycle = [
      "base",
      "delegating",
      "tee",
      "tee",
      "tee",
      "settling",
    ] as const;
    const delays = [2500, 1800, 4000, 4000, 4000, 2000];
    let i = 0,
      t: NodeJS.Timeout;
    function next() {
      i = (i + 1) % cycle.length;
      setState(cycle[i]);
      t = setTimeout(next, delays[i]);
    }
    t = setTimeout(next, delays[0]);
    return () => clearTimeout(t);
  }, []);

  const cfg = {
    base: { label: "On-Chain", color: "text-[#94a3b8]", dot: "bg-[#94a3b8]" },
    delegating: {
      label: "Delegating…",
      color: "text-[#f59e0b]",
      dot: "bg-[#f59e0b]",
    },
    tee: {
      label: "TEE Encrypted",
      color: "text-[#3b82f6]",
      dot: "bg-[#3b82f6]",
    },
    settling: {
      label: "Settling…",
      color: "text-[#10b981]",
      dot: "bg-[#10b981]",
    },
  }[state];

  const blur = state === "tee";
  const pnl = (mark - 90.24) * 1.5;

  return (
    <div className="flex flex-col items-center gap-3">
      <div className="flex items-center gap-2 text-[9px] font-bold uppercase tracking-widest text-[#4a5568]">
        TEE Privacy Demo
      </div>
      <table className="text-[11px] font-mono border border-white/[0.06] rounded overflow-hidden">
        <tbody>
          <tr className="bg-[#0d1117]">
            <Td className="font-bold text-[#e2e8f0]">SOL-PERP</Td>
            <Td>
              <span className="px-1.5 py-0.5 rounded-[3px] font-bold bg-[#064e3b] text-[#34d399]">
                LONG
              </span>
            </Td>
            <Td className="text-[#94a3b8]">1.50</Td>
            <Td className="text-[#94a3b8]">$90.24</Td>
            <Td
              className={`text-[#94a3b8] transition-all duration-500 ${blur ? "blur-sm select-none" : ""}`}
            >
              ${mark.toFixed(3)}
            </Td>
            <Td
              className={`font-bold transition-all duration-500 ${pnl >= 0 ? "text-[#10b981]" : "text-[#ef4444]"} ${blur ? "blur-sm select-none" : ""}`}
            >
              {pnl >= 0 ? "▲" : "▼"} ${Math.abs(pnl).toFixed(2)}
            </Td>
            <Td>
              <div
                className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded-[3px] border border-white/10 ${cfg.color} text-[9px] font-bold uppercase tracking-wider`}
              >
                <div
                  className={`w-1.5 h-1.5 rounded-full ${cfg.dot} ${state === "tee" ? "animate-pulse" : ""}`}
                />
                {cfg.label}
              </div>
            </Td>
          </tr>
        </tbody>
      </table>
      <span className="text-[9px] text-[#4a5568]">
        Connect wallet to open real positions
      </span>
    </div>
  );
}
