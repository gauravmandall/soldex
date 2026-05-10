// Soldex Engine — Persistent WebSocket Test Client
// Node 24 built-in WebSocket — no npm needed
// Usage: node ws_test.mjs [ping|open|delegate|close]
import { readFileSync } from "fs";
import { homedir } from "os";

const WS_URL       = "ws://localhost:9000/ws";
const MODE         = process.argv[2] || "ping";
const OWNER_PUBKEY = process.env.OWNER_PUBKEY || "11111111111111111111111111111111";
const POSITION_PDA = process.env.POSITION_PDA || "11111111111111111111111111111111";
const MARKET_ID    = process.env.MARKET_ID    || "SOL-PERP";
const NONCE        = Number(process.env.NONCE || "0");
const KEYPAIR_PATH = process.env.KEYPAIR_PATH || `${homedir()}/.config/solana/id.json`;
const REQUEST_ID   = `test-${Date.now()}`;

// ─── Base58 ───────────────────────────────────────────────────────────────────
const B58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
function b58decode(s) {
  let n = 0n;
  for (const c of s) { const d = B58.indexOf(c); if (d < 0) throw new Error(`bad b58: ${c}`); n = n * 58n + BigInt(d); }
  const hex = n.toString(16).padStart(64, "0");
  const out = new Uint8Array(hex.length / 2);
  for (let i = 0; i < out.length; i++) out[i] = parseInt(hex.slice(i*2, i*2+2), 16);
  let lead = 0; for (const c of s) { if (c === "1") lead++; else break; }
  const res = new Uint8Array(lead + out.length); res.set(out, lead); return res;
}
function b58encode(bytes) {
  let n = 0n; for (const b of bytes) n = n * 256n + BigInt(b);
  let s = ""; while (n > 0n) { const r = n % 58n; s = B58[Number(r)] + s; n = n / 58n; }
  for (const b of bytes) { if (b !== 0) break; s = "1" + s; } return s;
}

// ─── Ed25519 sign ─────────────────────────────────────────────────────────────
async function ed25519Sign(messageBytes, seed32) {
  const { sign } = await import("@noble/ed25519");
  return await sign(messageBytes, seed32.slice(0, 32));
}

// ─── Sign versioned tx ────────────────────────────────────────────────────────
// Layout: [num_sigs: u8][sig0: 64][sig1..n: 64 each][message...]
async function signVersionedTx(txBase64, keypairPath) {
  const raw = Buffer.from(txBase64, "base64");
  const kp  = JSON.parse(readFileSync(keypairPath, "utf8"));
  const secret = new Uint8Array(kp); // 64 bytes
  const seed   = secret.slice(0, 32);
  const numSigs = raw[0];
  const msgOff  = 1 + numSigs * 64;
  const msg     = raw.slice(msgOff);
  const sig     = await ed25519Sign(msg, seed);
  const out = Buffer.alloc(raw.length);
  out[0] = numSigs;
  Buffer.from(sig).copy(out, 1);
  raw.copy(out, 65, 65, msgOff);   // remaining sig slots
  raw.copy(out, msgOff, msgOff);   // message unchanged
  return out.toString("base64");
}

// ─── PDA derivation ───────────────────────────────────────────────────────────
// seeds = [b"position", market_id_16, owner_32, nonce_1]
async function isOnCurve(b) {
  try { await crypto.subtle.importKey("raw", b, { name: "Ed25519" }, false, ["verify"]); return true; }
  catch { return false; }
}
async function derivePositionPda(marketId, ownerPubkey, nonce) {
  const PROG = b58decode("7bWbam1aYjH42WLWJWWRM84Y2WXoknUwjbb6ZpATTM6k");
  const mkt  = new Uint8Array(16); mkt.set(new TextEncoder().encode(marketId).slice(0, 16));
  const own  = b58decode(ownerPubkey);
  const nn   = new Uint8Array([nonce]);
  const pfx  = new TextEncoder().encode("ProgramDerivedAddress");
  const seed = new TextEncoder().encode("position");
  for (let bump = 255; bump >= 0; bump--) {
    const data = new Uint8Array([...pfx, ...seed, ...mkt, ...own, ...nn, bump, ...PROG]);
    const hash = new Uint8Array(await crypto.subtle.digest("SHA-256", data));
    if (!(await isOnCurve(hash))) return { pda: b58encode(hash), bump };
  }
  throw new Error("no valid PDA bump");
}

// ─── Simple message map ───────────────────────────────────────────────────────
const SIMPLE = {
  ping: { type: "ping" },
  delegate: { type: "delegate_position", request_id: REQUEST_ID, owner_pubkey: OWNER_PUBKEY, position_pda: POSITION_PDA, market_id: MARKET_ID, nonce: NONCE },
  close:    { type: "close_and_undelegate_position", request_id: REQUEST_ID, owner_pubkey: OWNER_PUBKEY, position_pda: POSITION_PDA, market_id: MARKET_ID },
};

// ─── Open mode ────────────────────────────────────────────────────────────────
async function runOpen() {
  const kp     = JSON.parse(readFileSync(KEYPAIR_PATH, "utf8"));
  const owner  = b58encode(new Uint8Array(kp).slice(32));
  const SIDE   = process.env.SIDE       || "long";
  const SIZE   = Number(process.env.SIZE       || "1000000");
  const LEV    = Number(process.env.LEVERAGE   || "2");
  const COL    = Number(process.env.COLLATERAL || "1");
  const N      = Number(process.env.NONCE      || "0");

  console.log(`\n🔌 Connecting to ${WS_URL}`);
  console.log(`📤 Mode: open  owner=${owner}  ${SIDE} ${SIZE} ${MARKET_ID} x${LEV}  col=${COL} USDC  nonce=${N}\n`);

  const ws = new WebSocket(WS_URL);
  let step = "build";

  ws.addEventListener("open", () => {
    console.log("✅ Connected\n");
    ws.send(JSON.stringify({ type: "build_perps_order", market_id: MARKET_ID, side: SIDE, size: SIZE, leverage: LEV, collateral_usdc: COL, owner_pubkey: owner }));
  });

  ws.addEventListener("message", async ({ data }) => {
    const ts = new Date().toISOString();
    let msg; try { msg = JSON.parse(data); } catch { console.log(ts, "RAW:", data); return; }
    console.log(`[${ts}] ← ${msg.type}`);
    console.log(JSON.stringify(msg, null, 2));
    console.log("─".repeat(60));

    if (msg.type === "unsigned_tx" && step === "build") {
      step = "submit";
      console.log("\n✍️  Signing...");
      try {
        const signed = await signVersionedTx(msg.tx_base64, KEYPAIR_PATH);
        console.log("✅ Signed. Submitting...\n");
        ws.send(JSON.stringify({ type: "submit_transaction", tx_base64: signed }));
      } catch(e) { console.error("❌ Sign failed:", e.message); ws.close(); process.exit(1); }
    }

    if (msg.type === "order_ack" && step === "submit") {
      step = "done";
      const sig = msg.client_order_id;
      console.log(`\n✅ TX submitted: ${sig}`);
      console.log(`   https://explorer.solana.com/tx/${sig}?cluster=devnet\n`);
      try {
        const { pda, bump } = await derivePositionPda(MARKET_ID, owner, N);
        console.log("━".repeat(66));
        console.log(`✅ POSITION OPENED`);
        console.log(`   TX SIG:       ${sig}`);
        console.log(`   POSITION PDA: ${pda}  (bump=${bump})`);
        console.log(`   OWNER:        ${owner}`);
        console.log(`\n📋 Copy-paste for delegate:`);
        console.log(`   OWNER_PUBKEY=${owner} \\`);
        console.log(`   POSITION_PDA=${pda} \\`);
        console.log(`   MARKET_ID=${MARKET_ID} \\`);
        console.log(`   NONCE=${N} \\`);
        console.log(`   node engine/ws_test.mjs delegate`);
        console.log("━".repeat(66));
      } catch(e) { console.error("⚠️  PDA derive failed:", e.message, "(tx still ok — check explorer)"); }
      ws.close(); process.exit(0);
    }

    if (msg.type === "error") { console.error(`\n❌ [${msg.code}]: ${msg.message}`); ws.close(); process.exit(1); }
  });

  ws.addEventListener("error", e => { console.error("❌ WS:", e.message||e); process.exit(1); });
  ws.addEventListener("close", e => { console.log(`\n🔌 closed (${e.code})`); });
}

async function runDeposit() {
  const kp     = JSON.parse(readFileSync(KEYPAIR_PATH, "utf8"));
  const owner  = b58encode(new Uint8Array(kp).slice(32));
  const AMOUNT = Number(process.env.AMOUNT || "10"); // USDC

  console.log(`\n🔌 Connecting to ${WS_URL}`);
  console.log(`📤 Mode: deposit  owner=${owner}  amount=${AMOUNT} USDC  market=${MARKET_ID}\n`);

  const ws = new WebSocket(WS_URL);
  let step = "build";

  ws.addEventListener("open", () => {
    console.log("✅ Connected\n");
    ws.send(JSON.stringify({
      type: "deposit_collateral",
      market_id: MARKET_ID,
      amount_usdc: AMOUNT,
      owner_pubkey: owner,
    }));
  });

  ws.addEventListener("message", async ({ data }) => {
    const ts = new Date().toISOString();
    let msg; try { msg = JSON.parse(data); } catch { console.log(ts, "RAW:", data); return; }
    console.log(`[${ts}] ← ${msg.type}`);
    console.log(JSON.stringify(msg, null, 2));
    console.log("─".repeat(60));

    if (msg.type === "unsigned_tx" && step === "build") {
      step = "submit";
      console.log("\n✍️  Signing...");
      try {
        const signed = await signVersionedTx(msg.tx_base64, KEYPAIR_PATH);
        console.log("✅ Signed. Submitting...\n");
        ws.send(JSON.stringify({ type: "submit_transaction", tx_base64: signed }));
      } catch(e) { console.error("❌ Sign failed:", e.message); ws.close(); process.exit(1); }
    }

    if (msg.type === "order_ack" && step === "submit") {
      step = "done";
      console.log(`\n✅ Deposit confirmed: ${msg.client_order_id}`);
      console.log(`   https://explorer.solana.com/tx/${msg.client_order_id}?cluster=devnet\n`);
      console.log("━".repeat(66));
      console.log(`✅ MARGIN ACCOUNT INITIALIZED + FUNDED`);
      console.log(`   Now run: node engine/ws_test.mjs open`);
      console.log("━".repeat(66));
      ws.close(); process.exit(0);
    }

    if (msg.type === "error") { console.error(`\n❌ [${msg.code}]: ${msg.message}`); ws.close(); process.exit(1); }
  });

  ws.addEventListener("error", e => { console.error("❌ WS:", e.message||e); process.exit(1); });
  ws.addEventListener("close", e => { console.log(`\n🔌 closed (${e.code})`); });
}


// ─── Main ─────────────────────────────────────────────────────────────────────
if (MODE === "open") {
  runOpen().catch(e => { console.error(e); process.exit(1); });
} else if (MODE === "deposit") {        // ← ADD THIS
  runDeposit().catch(e => { console.error(e); process.exit(1); });
}  else if (MODE === "delegate") {
  runDelegate().catch(e => { console.error(e); process.exit(1); });
} else if (MODE === "activate") {
  runActivate().catch(e => { console.error(e); process.exit(1); });
} else if (MODE === "close") {
  runClose().catch(e => { console.error(e); process.exit(1); });
} else {
  const payload = SIMPLE[MODE];
  if (!payload) { console.error(`Unknown mode: ${MODE}. Use: ping | open | delegate | close`); process.exit(1); }
  console.log(`\n🔌 Connecting to ${WS_URL}\n📤 Mode: ${MODE}\n📦 Sending:`, JSON.stringify(payload, null, 2), `\n⏳ Waiting...\n`);
  const ws = new WebSocket(WS_URL);
  ws.addEventListener("open", () => { console.log("✅ Connected\n"); ws.send(JSON.stringify(payload)); });
  ws.addEventListener("message", ({ data }) => {
    const ts = new Date().toISOString();
    let p; try { p = JSON.parse(data); } catch { console.log(ts, "RAW:", data); return; }
    console.log(`[${ts}] ← ${p.type||"unknown"}`); console.log(JSON.stringify(p, null, 2)); console.log("─".repeat(60));
    if (["pong","error"].includes(p.type) && MODE === "ping") { console.log("\n✅ Done."); ws.close(); process.exit(0); }
  });
  ws.addEventListener("error", e => { console.error("❌ WS:", e.message||e); process.exit(1); });
  ws.addEventListener("close", e => { console.log(`\n🔌 closed (${e.code})`); process.exit(0); });
  setInterval(() => process.stdout.write("."), 10_000);
}

async function runActivate() {
  const SIZE       = Number(process.env.SIZE        || "1000000");
  const ENTRY      = Number(process.env.ENTRY_PRICE || "0");
  const IS_LONG    = (process.env.SIDE || "long") === "long";
  const COLLATERAL = Number(process.env.COLLATERAL  || "1000000"); // micro-USDC

  console.log(`\n🔌 Connecting to ${WS_URL}\n📤 Mode: activate\n`);
  console.log(`   owner=${OWNER_PUBKEY}`);
  console.log(`   position=${POSITION_PDA}`);
  console.log(`   market=${MARKET_ID}  nonce=${NONCE}\n`);

  const ws = new WebSocket(WS_URL);
  ws.addEventListener("open", () => {
    console.log("✅ Connected\n");
    ws.send(JSON.stringify({
      type: "activate_session",
      request_id: REQUEST_ID,
      owner_pubkey: OWNER_PUBKEY,
      position_pda: POSITION_PDA,
      market_id: MARKET_ID,
      nonce: NONCE,
      size: SIZE,
      entry_price: ENTRY,
      is_long: IS_LONG,
      collateral: COLLATERAL,
    }));
  });

  ws.addEventListener("message", async ({ data }) => {
    let msg; try { msg = JSON.parse(data); } catch { return; }
    console.log(`← ${msg.type}:`, JSON.stringify(msg, null, 2));

    if (msg.type === "session_activated") {
      console.log("\n━".repeat(66));
      console.log(`✅ SESSION ACTIVATED`);
      console.log(`   owner:    ${msg.owner_pubkey}`);
      console.log(`   position: ${msg.position_pda}`);
      console.log(`\n📋 Engine is now tracking this position on PER`);
      console.log(`   Price pump will update mark price every 2s`);
      console.log(`   Keeper will liquidate if margin < 5%`);
      console.log(`\n📋 Copy-paste for close:`);
      console.log(`   OWNER_PUBKEY=${OWNER_PUBKEY} \\`);
      console.log(`   POSITION_PDA=${POSITION_PDA} \\`);
      console.log(`   MARKET_ID=${MARKET_ID} \\`);
      console.log(`   node engine/ws_test.mjs close`);
      console.log("━".repeat(66));
      ws.close(); process.exit(0);
    }

    if (msg.type === "session_activation_failed") {
      console.error(`\n❌ [${msg.code}]: ${msg.message}`);
      ws.close(); process.exit(1);
    }

    if (msg.type === "error") {
      console.error(`\n❌ [${msg.code}]: ${msg.message}`);
      ws.close(); process.exit(1);
    }
  });

  ws.addEventListener("error", e => { console.error("❌ WS:", e.message||e); process.exit(1); });
}

async function runClose() {
  console.log(`\n🔌 Connecting to ${WS_URL}\n📤 Mode: close\n`);
  console.log(`   owner=${OWNER_PUBKEY}`);
  console.log(`   position=${POSITION_PDA}`);
  console.log(`   market=${MARKET_ID}\n`);

  const ws = new WebSocket(WS_URL);
  ws.addEventListener("open", () => {
    console.log("✅ Connected\n");
    ws.send(JSON.stringify({
      type: "close_and_undelegate_position",
      request_id: REQUEST_ID,
      owner_pubkey: OWNER_PUBKEY,
      position_pda: POSITION_PDA,
      market_id: MARKET_ID,
    }));
  });

  ws.addEventListener("message", async ({ data }) => {
    let msg; try { msg = JSON.parse(data); } catch { return; }
    console.log(`← ${msg.type}:`, JSON.stringify(msg, null, 2));

    if (msg.type === "close_er_submitted") {
      console.log(`\n✅ Position closed on PER: ${msg.signature}`);
      console.log(`   Waiting for undelegate tx...`);
    }

    if (msg.type === "unsigned_undelegate_tx") {
      console.log(`\n📋 Undelegate tx received — sign and submit to ER RPC`);
      console.log(`   tx_base64: ${msg.tx_base64.slice(0, 40)}...`);
      console.log(`   submit_to: ${msg.submit_to}`);
      console.log("\n━".repeat(66));
      console.log(`✅ POSITION CLOSED + UNDELEGATED`);
      console.log(`   Position returned to Solana base layer`);
      console.log(`   Session cleanup triggered`);
      console.log("━".repeat(66));
      ws.close(); process.exit(0);
    }

    if (msg.type === "error") {
      console.error(`\n❌ [${msg.code}]: ${msg.message}`);
      ws.close(); process.exit(1);
    }
  });

  // Timeout — close + undelegate can take a few seconds
  setTimeout(() => {
    console.error("\n⏱️  Timeout waiting for close response (15s)");
    ws.close(); process.exit(1);
  }, 15_000);

  ws.addEventListener("error", e => { console.error("❌ WS:", e.message||e); process.exit(1); });
}

async function runDelegate() {
  const kp = JSON.parse(readFileSync(KEYPAIR_PATH, "utf8"));
  const ws = new WebSocket(WS_URL);
  console.log(`\n🔌 Connecting to ${WS_URL}\n📤 Mode: delegate\n`);
  ws.addEventListener("open", () => {
    console.log("✅ Connected\n");
    ws.send(JSON.stringify({
      type: "delegate_position",
      request_id: REQUEST_ID,
      owner_pubkey: OWNER_PUBKEY,
      position_pda: POSITION_PDA,
      market_id: MARKET_ID,
      nonce: NONCE,
    }));
  });
  ws.addEventListener("message", async ({ data }) => {
    let msg; try { msg = JSON.parse(data); } catch { return; }
    if (!["unsigned_delegate_tx","error"].includes(msg.type)) return;
    console.log(`← ${msg.type}:`, JSON.stringify(msg, null, 2));
    if (msg.type === "error") { console.error(`❌ ${msg.code}: ${msg.message}`); ws.close(); process.exit(1); }
    if (msg.type === "unsigned_delegate_tx") {
      console.log("\n✍️  Signing delegation tx...");
      const signed = await signVersionedTx(msg.tx_base64, KEYPAIR_PATH);
      ws.send(JSON.stringify({ type: "submit_transaction", tx_base64: signed }));
      console.log("✅ Delegation tx submitted!");
      ws.close(); process.exit(0);
    }
  });
  ws.addEventListener("error", e => { console.error("❌ WS:", e.message||e); process.exit(1); });
}
