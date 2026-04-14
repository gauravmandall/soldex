#!/bin/bash

# SOLDEX — Local Desktop Launcher
# Starts the Trading Engine (Rust) and the Dashboard (Next.js)
# Inspired by the Polymarket-Metsumi launcher

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

log() { echo -e "${CYAN}[SOLDEX]${NC} $1"; }
success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; }

# 1. Environment check
if [ ! -f .env ]; then
    error "Missing .env file in root. Please create one based on .env.example."
    exit 1
fi

# 2. Ports check
log "Cleaning up existing processes on ports 9000 (Engine) and 3000 (Web)..."
lsof -ti:9000 | xargs kill -9 2>/dev/null || true
lsof -ti:3000 | xargs kill -9 2>/dev/null || true

# 3. Build & Start Engine
log "Building Soldex Trading Engine (Rust)..."
# Build the engine using its manifest path
cargo build --release --manifest-path engine/Cargo.toml

# Locate binary
ENGINE_BIN="./engine/target/release/soldex-engine"
if [ ! -f "$ENGINE_BIN" ]; then
    # Fallback to shared target folder if configured
    ENGINE_BIN="./target/release/soldex-engine"
fi

log "Starting Trading Engine..."
$ENGINE_BIN > engine.log 2>&1 &
ENGINE_PID=$!
success "Engine started (PID: $ENGINE_PID). Logs: tail -f engine.log"

# 4. Start Frontend
log "Syncing environment to web apps..."
cp .env apps/web/.env.local

log "Starting Next.js Dashboard..."
# Determine package manager
if command -v pnpm &> /dev/null; then
    pnpm --filter @soldex/web dev > web.log 2>&1 &
    WEB_PID=$!
else
    log "pnpm not found, falling back to npm (this may take longer)..."
    (cd apps/web && npm install && npm run dev) > web.log 2>&1 &
    WEB_PID=$!
fi

success "Dashboard starting at http://localhost:3000"
log "🚀 System ready. Press Ctrl+C to stop all services."
echo "--------------------------------------------------"

# Handle cleanup
trap "echo -e '\n${YELLOW}Shutting down...${NC}'; kill $ENGINE_PID $WEB_PID 2>/dev/null || true; exit" INT

# Stream engine logs to console
tail -f engine.log
