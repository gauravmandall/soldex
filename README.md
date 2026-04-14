# Soldex.fi

**Privacy-First Perpetuals DEX on Solana**

Execute perps orders and discover Polymarket opportunities with self-custody wallets and built-in privacy rotation. No browser extension required.

---

## ⚡️ Quick Commands

| Task | Command |
|------|---------|
| **Full Local Start** | `pnpm dev` |
| **Frontend Only** | `cd apps/web && pnpm dev` |
| **Run All Local** | `./run_local.sh` |
| **Engine (Rust)** | `cd engine && cargo run` |
| **Build Project** | `pnpm build` |
| **Clean Install** | `pnpm install` |

---

## Architecture

```
soldex/
├── apps/web/              # Next.js 14 frontend (TypeScript)
│   ├── app/               # App Router pages
│   │   ├── page.tsx       # Trading terminal (SOL-PERP default)
│   │   ├── perps/[market] # Dynamic market pages
│   │   ├── polymarket/    # Polymarket discovery
│   │   ├── portfolio/     # Positions + wallet history
│   │   └── rotate/        # Privacy rotation wizard
│   ├── components/
│   │   ├── terminal/      # TradingTerminal, OrderBook, OrderPanel, etc.
│   │   ├── charts/        # CandleChart (lightweight-charts)
│   │   ├── wallet/        # SelfCustodyWalletButton
│   │   └── polymarket/    # PolymarketPanel
│   ├── hooks/
│   │   ├── useEngineWS.ts      # WebSocket connection to Rust engine
│   │   ├── useMarketStore.ts   # Zustand store for market data
│   │   └── useSelfCustodyWallet.ts  # Keypair generation & rotation
│   └── providers/
│
├── engine/                # Rust async engine
│   └── src/
│       ├── main.rs        # Axum WebSocket server + task orchestration
│       ├── config.rs      # Environment config
│       ├── orderbook/     # Price-time priority matching engine
│       ├── perps/         # Position management, price feeds
│       ├── polymarket/    # Polymarket CLOB bridge (REST + WS)
│       ├── risk/          # Margin checks, liquidation logic
│       ├── wallet/        # AES-GCM encrypted keypair storage
│       └── ws/            # Client message protocol & routing
│
├── programs/soldex-perps/ # Anchor program (Rust on-chain)
│   └── src/
│       ├── lib.rs         # Program entry + instruction routing
│       ├── state/         # MarketState, MarginAccount, Position PDAs
│       ├── instructions/  # open_position, close_position, liquidate…
│       └── errors.rs      # Custom error codes
│
└── packages/sdk/          # TypeScript SDK (@soldex/sdk)
    └── src/
        ├── client.ts      # SoldexClient (AnchorProvider + PDA helpers)
        ├── perps.ts       # PerpsSDK (build txs, fetch positions)
        ├── wallet.ts      # Self-custody keypair utilities
        └── polymarket.ts  # Polymarket order helpers
```

---

## Key Design Decisions

### Self-Custody Wallet (No Extensions)
Users generate a Solana keypair entirely in the browser via `@solana/web3.js`. The private key is stored in `sessionStorage` only — cleared on tab close. No Phantom, no Backpack, no extension fingerprinting.

```ts
// Generate fresh wallet
const { generate, wallet } = useSelfCustodyWallet()
await generate('My Trading Wallet')

// Sign and submit a transaction
const signed = sign(unsignedTx)
await connection.sendRawTransaction(signed.serialize())
```

### Privacy Rotation
Every 30 days, the frontend prompts users to rotate to a fresh keypair. The old wallet retires. No on-chain link between old and new addresses — providing the same privacy guarantee as withdrawing to a new CEX account.

```
Day 0:  Wallet A — start trading
Day 30: Wallet B — generated, A retired
Day 60: Wallet C — generated, B retired
```

On-chain: three unlinked wallets. Off-chain: your session history.

### Rust Engine (WebSocket Protocol)
The frontend communicates with the Rust engine over a persistent WebSocket. The engine:
- Runs the in-memory orderbook (price-time priority)
- Bridges to Polymarket CLOB API (polls every 5s)
- Polls price feeds (Jupiter Price API → swap for Pyth in production)
- Builds unsigned Solana transactions → returns to frontend for signing
- Never holds private keys (or optionally does, AES-GCM encrypted)

```json
// Client → Engine
{ "type": "build_perps_order", "market_id": "SOL-PERP", "side": "long",
  "size": 10, "leverage": 5, "collateral_usdc": 300, "owner_pubkey": "..." }

// Engine → Client
{ "type": "unsigned_tx", "request_id": "...", "tx_base64": "...",
  "description": "Open long 10 SOL-PERP x5" }
```

### Anchor Program: soldex-perps
Four core PDAs:
- `MarketState` — global market config (tick size, leverage limits, OI)
- `MarginAccount` — per-user per-market collateral (USDC)
- `Position` — individual open position with entry price, size, funding index

---

## Getting Started

### Prerequisites
- Node.js 20+ / pnpm 9+
- Rust 1.79+ (`rustup update stable`)
- Solana CLI 1.18+
- Anchor 0.30+ (`cargo install --git https://github.com/coral-xyz/anchor anchor-cli`)

### 1. Install dependencies
```bash
pnpm install
```

### 2. Start the Rust engine
```bash
cp engine/.env.example engine/.env
# Edit engine/.env with your Helius RPC key and a random WALLET_ENCRYPTION_KEY

cd engine
cargo run
# Engine starts on ws://localhost:9000/ws
```

### 3. Start the frontend
```bash
cp apps/web/.env.local.example apps/web/.env.local
# Edit NEXT_PUBLIC_RPC_URL with your Helius key

pnpm dev
# Frontend on http://localhost:3000
```

### 4. Deploy the Anchor program (devnet)
```bash
solana config set --url devnet
solana airdrop 2

anchor build
anchor deploy --provider.cluster devnet

# Copy the program ID into:
#   - Anchor.toml
#   - engine/.env  (PROGRAM_ID)
#   - apps/web/.env.local  (NEXT_PUBLIC_PROGRAM_ID)
#   - programs/soldex-perps/src/lib.rs  (declare_id!)
```

---

## WebSocket Message Reference

### Client → Engine
| Type | Description |
|------|-------------|
| `subscribe` | Subscribe to market orderbook + trades |
| `place_order` | Place limit or market order |
| `cancel_order` | Cancel a resting order |
| `build_perps_order` | Request unsigned tx for perps position |
| `polymarket_order` | Execute a Polymarket CLOB order |
| `get_poly_opportunities` | Fetch live Polymarket markets |
| `get_snapshot` | Get full orderbook snapshot |
| `ping` | Heartbeat |

### Engine → Client
| Type | Description |
|------|-------------|
| `snapshot` | Full orderbook for a market |
| `ticker_update` | Price/volume/funding update (every 2s) |
| `fill_update` | Trade execution |
| `order_ack` | Order acknowledgment |
| `polymarket_opportunities` | Discovered Poly markets |
| `unsigned_tx` | Unsigned Solana tx for frontend signing |
| `error` | Error response |
| `pong` | Heartbeat reply |

---

## Privacy Model

| Threat | Protection | Limitation |
|--------|-----------|------------|
| On-chain wallet linkage | 30-day key rotation | Old positions tied to old key |
| Browser extension fingerprinting | No wallet adapter required | N/A |
| Engine knows your trades | Self-sign + submit | Engine sees order intent |
| Network correlation | — | Use Tor/VPN separately |
| Session persistence | sessionStorage only | Lost on tab close |

**North star:** Full ZK-based rotation proofs (Groth16) in a future version. For v0.1, manual rotation achieves practical unlinkability for most threat models.

---

## Tech Stack

| Layer | Tech |
|-------|------|
| Frontend | Next.js 14, TypeScript, Tailwind CSS |
| State | Zustand + Immer |
| Charts | lightweight-charts (TradingView) |
| Wallet | @solana/web3.js, tweetnacl (no adapter) |
| Engine | Rust, Tokio, Axum, Serde |
| Matching | Custom price-time priority, DashMap |
| On-chain | Anchor 0.30, Solana 1.18 |
| Oracle | Pyth (stub → real in production) |
| Polymarket | CLOB REST API + WebSocket |
| Deployment | Vercel (frontend), Fly.io / Render (engine) |

---

## Roadmap

- [ ] Pyth price feed integration (replace stubs)
- [ ] Full close_position + liquidate instructions
- [ ] USDC SPL token collateral (vs SOL)
- [ ] Funding rate keeper bot (cron)
- [ ] Polymarket EIP-712 signature flow from browser
- [ ] ZK wallet rotation proofs (Groth16 / PLONK)
- [ ] Mobile responsive layout
- [ ] Indexer (Postgres event log)
- [ ] Mainnet deployment

---

*Privacy by design. Self-custody by default. Ship it.*

**Soldex — Trade freely. Stay private.**
