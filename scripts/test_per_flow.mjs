/**
 * test_per_flow.mjs — Full end-to-end PER integration test
 *
 * Correct flow (two-phase open):
 *  1. Deposit collateral          (base layer)
 *  2. Init position               (base layer) ← was openPosition with wrong params
 *  3. Delegate position to ER     (base layer)
 *  4. Open position on ER         (ER)         ← NEW: sets price + OI
 *  5. Funding tick                (ER)
 *  6. Close position on ER        (ER)         ← fixed params + market account
 *  7. Undelegate position         (ER → base)
 *
 * Usage:
 *   node scripts/test_per_flow.mjs
 */

import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import pkg from "@coral-xyz/anchor";
const { Program, AnchorProvider, Wallet, BN } = pkg;
import {
  getOrCreateAssociatedTokenAccount,
  mintTo,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { readFileSync } from "fs";
import { homedir } from "os";

// ─── Config ───────────────────────────────────────────────────────────────────

const BASE_RPC = "https://api.devnet.solana.com";
const TEE_RPC = "https://devnet-tee.magicblock.app";

const PROGRAM_ID = new PublicKey(
  "F8NaJJsJfScnUtz5mHvLfteU6i27n9zrS2dVWxaGvPHR",
);
const QUOTE_MINT = new PublicKey(
  "6kgSZ26MzBqHQBZf6f6HxRwFLMFZzXgDj2j4vs11MTRB",
);
const PYTH_FEED = new PublicKey("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE");
const MARKET_PDA = new PublicKey(
  "3CC7QD3CsaHu4BVQyyMUjjaGTD5MNwnDpZLLzQobzNNF",
);
const MARKET_ID_STR = "SOL-PERP";

const DELEGATION_PROGRAM_ID = new PublicKey(
  "DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh",
);
const PERMISSION_PROGRAM_ID = new PublicKey(
  "ACLseoPoyC3cBqoUtkbjZ4aDrkurZW86v19pXz2XQnp1",
);
const MAGIC_PROGRAM_ID = new PublicKey(
  "Magic11111111111111111111111111111111111111",
);
const MAGIC_CONTEXT_ACCOUNT = new PublicKey(
  "MagicContext1111111111111111111111111111111",
);

// Position params — change these to adjust the test trade
const COLLATERAL_DEPOSIT = 100_000_000; // 100 USDC (6 decimals)
const POS_COLLATERAL = 100_000_000; // $100 — covers margin on $930 notional
const POS_SIZE = 10_000_000; // 10 SOL lots (= $930 notional @ $93)
const POS_LEVERAGE_BPS = 1_000;
const MARK_PRICE = 93_000_000; // $93 (6 decimals) — passed to ER calls
const NONCE = Math.floor(Date.now() / 1000) % 256; // u8, unique per ~4 min window

// ─── Keypair ──────────────────────────────────────────────────────────────────

const kp = JSON.parse(
  readFileSync(`${homedir()}/.config/solana/id.json`, "utf8"),
);
const admin = Keypair.fromSecretKey(new Uint8Array(kp));

// ─── PDA helpers ──────────────────────────────────────────────────────────────

function marketIdBytes(id) {
  const buf = Buffer.alloc(16, 0);
  Buffer.from(id).copy(buf);
  return Array.from(buf);
}

const marketId = marketIdBytes(MARKET_ID_STR);

const [vaultPDA] = PublicKey.findProgramAddressSync(
  [Buffer.from("vault"), Buffer.from(marketId)],
  PROGRAM_ID,
);
const [marginPDA] = PublicKey.findProgramAddressSync(
  [Buffer.from("margin"), Buffer.from(marketId), admin.publicKey.toBuffer()],
  PROGRAM_ID,
);
const [positionPDA] = PublicKey.findProgramAddressSync(
  [
    Buffer.from("position"),
    Buffer.from(marketId),
    admin.publicKey.toBuffer(),
    Buffer.from([NONCE]),
  ],
  PROGRAM_ID,
);
const [bufferPDA] = PublicKey.findProgramAddressSync(
  [Buffer.from("buffer"), positionPDA.toBuffer()],
  PROGRAM_ID,
);
const [delegationRecord] = PublicKey.findProgramAddressSync(
  [Buffer.from("delegation"), positionPDA.toBuffer()],
  DELEGATION_PROGRAM_ID,
);
const [delegationMetadata] = PublicKey.findProgramAddressSync(
  [Buffer.from("delegation-metadata"), positionPDA.toBuffer()],
  DELEGATION_PROGRAM_ID,
);
const [permissionPDA] = PublicKey.findProgramAddressSync(
  [Buffer.from("permission:"), positionPDA.toBuffer()],
  PERMISSION_PROGRAM_ID,
);

// ─── Logging helpers ──────────────────────────────────────────────────────────

const now = () => new Date().toISOString().slice(11, 23);
const ms = (t) => `${Date.now() - t}ms`;
const step = (n, title) =>
  console.log(`\n${"─".repeat(60)}\nSTEP ${n} — ${title}\n${"─".repeat(60)}`);
const ok = (msg, sig, lat) => {
  console.log(`[${now()}] ✅ ${msg}${lat ? ` | ${lat}` : ""}`);
  if (sig) console.log(`         sig: ${sig}`);
};
const warn = (msg) => console.log(`[${now()}] ⚠️  ${msg}`);
const fail = (msg, e) => {
  console.log(`[${now()}] ❌ ${msg}`);
  if (e?.logs) console.log("logs:\n" + e.logs.join("\n"));
};

// ─── Main ─────────────────────────────────────────────────────────────────────

async function main() {
  console.log("\n🚀 Soldex PER — End-to-End Flow Test");
  console.log("═".repeat(60));
  console.log(`Admin    : ${admin.publicKey}`);
  console.log(`Market   : ${MARKET_ID_STR} → ${MARKET_PDA}`);
  console.log(`Position : ${positionPDA}`);
  console.log(`Margin   : ${marginPDA}`);
  console.log("═".repeat(60));

  const baseConn = new Connection(BASE_RPC, "confirmed");
  const teeConn = new Connection(TEE_RPC, {
    commitment: "confirmed",
    wsEndpoint: "wss://api.devnet.solana.com",
  });
  const baseProvider = new AnchorProvider(baseConn, new Wallet(admin), {
    commitment: "confirmed",
  });
  const teeProvider = new AnchorProvider(teeConn, new Wallet(admin), {
    commitment: "confirmed",
  });

  const idl = JSON.parse(
    readFileSync("./target/idl/soldex_perps.json", "utf8"),
  );
  const baseProgram = new Program(idl, baseProvider);
  const teeProgram = new Program(idl, teeProvider);

  // ── Token account setup ───────────────────────────────────────────────────
  console.log("\n📋 Token account setup...");
  const traderATA = await getOrCreateAssociatedTokenAccount(
    baseConn,
    admin,
    QUOTE_MINT,
    admin.publicKey,
  );
  console.log(`   ATA     : ${traderATA.address}`);
  console.log(`   Balance : ${Number(traderATA.amount) / 1e6} USDC`);

  if (Number(traderATA.amount) < COLLATERAL_DEPOSIT) {
    try {
      await mintTo(
        baseConn,
        admin,
        QUOTE_MINT,
        traderATA.address,
        admin,
        COLLATERAL_DEPOSIT * 10,
      );
      console.log(`   ✅ Minted ${(COLLATERAL_DEPOSIT * 10) / 1e6} USDC`);
    } catch (e) {
      warn(
        `Cannot mint — not mint authority. Balance: ${Number(traderATA.amount) / 1e6} USDC`,
      );
    }
  }

  // ── STEP 1: Deposit Collateral ────────────────────────────────────────────
  step(1, "Deposit Collateral (base layer)");
  let t = Date.now();
  try {
    const sig = await baseProgram.methods
      .depositCollateral(marketId, new BN(COLLATERAL_DEPOSIT))
      .accounts({
        owner: admin.publicKey,
        margin: marginPDA,
        userTokenAccount: traderATA.address,
        vault: vaultPDA,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([admin])
      .rpc();
    ok(`Deposited ${COLLATERAL_DEPOSIT / 1e6} USDC`, sig, ms(t));
  } catch (e) {
    if (
      e.message?.includes("FundingAlreadySettled") ||
      e.message?.includes("0x1778")
    ) {
      warn("FundingAlreadySettled — already ticked this interval (correct)");
    } else {
      fail(`Funding tick failed: ${e.message}`, e);
      console.log("FULL ERROR:", JSON.stringify(e, null, 2));
      if (e.logs) console.log("PROGRAM LOGS:\n" + e.logs.join("\n"));
    }
  }

  // ── STEP 2: Init Position (base layer — phase 1) ──────────────────────────
  // Creates position PDA + deducts collateral. Does NOT touch market (delegated).
  // FIX: was openPosition({ sizeUsd, limitPrice }) — wrong instruction + wrong params.
  step(2, "Init Position (base layer)");
  t = Date.now();

  const posInfo = await baseConn.getAccountInfo(positionPDA);
  const positionExists = posInfo && posInfo.data.length > 8;

  if (positionExists) {
    warn(
      `Position PDA already exists (len=${posInfo.data.length}) — skipping initPosition`,
    );
  } else {
    try {
      const sig = await baseProgram.methods
        .initPosition({
          marketId: marketId,
          side: { long: {} },
          size: new BN(POS_SIZE), // ← was sizeUsd (wrong name)
          leverageBps: new BN(POS_LEVERAGE_BPS), // ← was limitPrice (wrong field)
          collateral: new BN(POS_COLLATERAL),
          nonce: NONCE,
        })
        .accounts({
          user: admin.publicKey,
          market: MARKET_PDA, // read-only — not mut
          margin: marginPDA,
          position: positionPDA,
          systemProgram: SystemProgram.programId,
          rent: SYSVAR_RENT_PUBKEY,
        })
        .signers([admin])
        .rpc();
      ok(
        "Position initialised (collateral deducted, not yet open)",
        sig,
        ms(t),
      );
    } catch (e) {
      fail(`Init position failed: ${e.message}`, e);
      process.exit(1);
    }
  }

  // ── STEP 3: Delegate Position to ER ──────────────────────────────────────
  step(3, "Delegate Position to ER (base layer)");
  t = Date.now();

  const delInfo = await baseConn.getAccountInfo(delegationRecord);
  if (delInfo) {
    warn("Already delegated — skipping");
  } else {
    try {
      const sig = await baseProgram.methods
        .delegatePosition(marketId, NONCE, admin.publicKey)
        .accounts({
          bufferPosition: bufferPDA,
          delegationRecordPosition: delegationRecord,
          delegationMetadataPosition: delegationMetadata,
          position: positionPDA,
          owner: admin.publicKey,
          permissionProgram: SystemProgram.programId,
          permission: permissionPDA,
          ownerProgram: PROGRAM_ID,
          delegationProgram: DELEGATION_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([admin])
        .rpc();
      ok("Position delegated to ER", sig, ms(t));
    } catch (e) {
      fail(`Delegate failed: ${e.message}`, e);
      process.exit(1);
    }
  }

  const posAfterDel = await baseConn.getAccountInfo(positionPDA);
  if (posAfterDel?.owner.toString() === DELEGATION_PROGRAM_ID.toString()) {
    ok(
      "Privacy verified — base layer owner = delegation program (live state hidden in TEE)",
    );
  } else {
    warn(`Position owner on base layer: ${posAfterDel?.owner}`);
  }

  // ── STEP 4: Open Position on ER (phase 2) ────────────────────────────────
  // FIX: this entire step was missing. Sets entry_price, entry_funding_index,
  // is_open=true, and updates market open interest — all on ER.
  step(4, "Open Position on ER (phase 2 — sets price + OI)");
  t = Date.now();
  try {
    const sig = await teeProgram.methods
      .openPositionEr(new BN(MARK_PRICE), marketId, NONCE)
      .accounts({
        owner: admin.publicKey,
        position: positionPDA,
        market: MARKET_PDA,
      })
      .signers([admin])
      .rpc();
    ok(`Position opened on ER @ $${MARK_PRICE / 1e6}`, sig, ms(t));
  } catch (e) {
    fail(`Open position ER failed: ${e.message}`, e);
    console.log("FULL ERROR:", JSON.stringify(e, null, 2)); // ← ADD
    process.exit(1);
  }

  // ── STEP 5: Funding Tick on ER ────────────────────────────────────────────
  step(5, "Funding Tick (ER)");
  t = Date.now();
  try {
    const sig = await teeProgram.methods
      .fundingTickEr(new BN(MARK_PRICE))
      .accounts({
        keeper: admin.publicKey,
        market: MARKET_PDA,
      })
      .signers([admin])
      .rpc();
    ok("Funding tick confirmed on ER", sig, ms(t));
  } catch (e) {
    if (
      e.message?.includes("FundingAlreadySettled") ||
      e.message?.includes("0x1778")
    ) {
      warn("FundingAlreadySettled — already ticked this interval (correct)");
    } else {
      // Drain the async logs from SendTransactionError
      let logs = e.logs ?? [];
      if (!logs.length && typeof e.getLogs === "function") {
        try {
          logs = await e.getLogs();
        } catch (_) {}
      }
      console.log(`[${now()}] ❌ Funding tick failed: ${e.message}`);
      console.log("Error name   :", e.name);
      console.log("Error code   :", e.code ?? e.error?.errorCode?.code ?? "—");
      console.log("Raw error    :", JSON.stringify(e, null, 2));
      if (logs.length) console.log("Program logs :\n" + logs.join("\n"));
      else console.log("(no program logs returned)");
    }
  }

  // ── STEP 6: Close Position on ER ─────────────────────────────────────────
  // FIX: was closePositionEr({ nonce, limitPrice }) — wrong params entirely.
  // Correct: (mark_price: u64, market_id: [u8;16], nonce: u8) + market account.
  step(6, "Close Position (ER)");
  t = Date.now();
  try {
    const sig = await teeProgram.methods
      .closePositionEr(new BN(MARK_PRICE), marketId, NONCE)
      .accounts({
        position: positionPDA,
        owner: admin.publicKey,
        market: MARKET_PDA, // ← was missing entirely
      })
      .signers([admin])
      .rpc();
    ok("Position closed on ER", sig, ms(t));
  } catch (e) {
    fail(`Close position ER failed: ${e.message}`, e);
    // Don't exit — still attempt undelegate
  }

  // ── STEP 7: Undelegate Position ───────────────────────────────────────────
  step(7, "Undelegate Position (ER → base layer)");
  t = Date.now();
  try {
    const sig = await teeProgram.methods
      .undelegatePosition(marketId, NONCE)
      .accounts({
        position: positionPDA,
        owner: admin.publicKey,
        magicProgram: MAGIC_PROGRAM_ID,
        magicContext: MAGIC_CONTEXT_ACCOUNT,
      })
      .signers([admin])
      .rpc();
    ok("Position undelegated — state committed back to Solana", sig, ms(t));
  } catch (e) {
    fail(`Undelegate failed: ${e.message}`, e);
  }

  // ── Summary ───────────────────────────────────────────────────────────────
  console.log("\n" + "═".repeat(60));
  console.log("📊 Final State");
  console.log("═".repeat(60));

  const mktFinal = await baseConn.getAccountInfo(MARKET_PDA);
  console.log(`Market owner  : ${mktFinal?.owner}`);
  if (mktFinal?.owner.toString() === DELEGATION_PROGRAM_ID.toString())
    console.log("✅ Market still delegated (funding ticks active)");

  const posFinal = await baseConn.getAccountInfo(positionPDA);
  if (!posFinal) {
    console.log("✅ Position closed and reclaimed");
  } else {
    console.log(`Position owner: ${posFinal.owner}`);
    if (posFinal.owner.toString() === PROGRAM_ID.toString())
      console.log("✅ Position back on base layer");
  }

  console.log("\n✅ PER end-to-end test complete\n" + "═".repeat(60) + "\n");
}

main().catch((e) => {
  console.error("\n❌ Fatal:", e.message);
  if (e.logs) console.error("Program logs:\n", e.logs.join("\n"));
  process.exit(1);
});
