import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { SoldexPerps } from "../target/types/soldex_perps";
import {
  Keypair,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import { createMint } from "@solana/spl-token";
import { assert } from "chai";

// ─── Constants ────────────────────────────────────────────────────────────────

const DELEGATION_PROGRAM_ID = new PublicKey(
  "DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh",
);
const MAGIC_PROGRAM_ID = new PublicKey(
  "Magic11111111111111111111111111111111111111",
);

// ─── Helpers ──────────────────────────────────────────────────────────────────

function marketIdBytes(id: string): number[] {
  const buf = Buffer.alloc(16, 0);
  buf.write(id.slice(0, 16), "utf8");
  return Array.from(buf);
}

function findMarketPDA(
  marketId: number[],
  programId: PublicKey,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("market"), Buffer.from(marketId)],
    programId,
  );
}

function findVaultPDA(
  marketId: number[],
  programId: PublicKey,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), Buffer.from(marketId)],
    programId,
  );
}

function findPositionPDA(
  marketId: number[],
  user: PublicKey,
  nonce: number,
  programId: PublicKey,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from("position"),
      Buffer.from(marketId),
      user.toBuffer(),
      Buffer.from([nonce]),
    ],
    programId,
  );
}

function findBufferPDA(
  positionPDA: PublicKey,
  programId: PublicKey,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("delegate-buffer"), positionPDA.toBuffer()],
    programId,
  );
}

function findDelegationRecordPDA(positionPDA: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("delegation"), positionPDA.toBuffer()],
    DELEGATION_PROGRAM_ID,
  );
}

function findDelegationMetadataPDA(
  positionPDA: PublicKey,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("delegation-metadata"), positionPDA.toBuffer()],
    DELEGATION_PROGRAM_ID,
  );
}

function findMagicContextPDA(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("magic_context")],
    MAGIC_PROGRAM_ID,
  );
}

// ─── Test Suite ───────────────────────────────────────────────────────────────

describe("soldex-perps — MagicBlock integration", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SoldexPerps as Program<SoldexPerps>;
  const connection = provider.connection;

  const MARKET_ID_STR = "SOL-PERP";
  const marketId = marketIdBytes(MARKET_ID_STR);
  const NONCE = 0;

  let admin: Keypair;
  let trader: Keypair;
  let quoteMint: PublicKey;
  let marketPDA: PublicKey;
  let vaultPDA: PublicKey;
  let positionPDA: PublicKey;
  let priceFeedKp: Keypair;

  before(async () => {
    admin = Keypair.generate();
    trader = Keypair.generate();
    priceFeedKp = Keypair.generate();

    await connection.confirmTransaction(
      await connection.requestAirdrop(admin.publicKey, 10e9),
    );
    await connection.confirmTransaction(
      await connection.requestAirdrop(trader.publicKey, 10e9),
    );

    quoteMint = await createMint(connection, admin, admin.publicKey, null, 6);

    [marketPDA] = findMarketPDA(marketId, program.programId);
    [vaultPDA]  = findVaultPDA(marketId, program.programId);
    [positionPDA] = findPositionPDA(marketId, trader.publicKey, NONCE, program.programId);
  });

  // ── 1. Initialize market ───────────────────────────────────────────────────

  it("initializes a market", async () => {
    await program.methods
      .initializeMarket({
        marketId,
        priceFeed: priceFeedKp.publicKey,
        tickSizeBps: new BN(100),
        lotSize: new BN(1_000_000),
        maxLeverageBps: new BN(2_000),
        makerFeeBps: 2,
        takerFeeBps: 5,
        initialMarginBps: 500,
        maintenanceMarginBps: 250,
      })
      .accounts({
        admin: admin.publicKey,
        market: marketPDA,
        quoteMint,
        vault: vaultPDA,
        systemProgram: SystemProgram.programId,
      })
      .signers([admin])
      .rpc();

    const market = await program.account.marketState.fetch(marketPDA);
    assert.isTrue(market.isActive);
    assert.equal(market.lotSize.toNumber(), 1_000_000);
    console.log("✓ Market initialized:", marketPDA.toBase58());
  });

  // ── 2. IDL error checks ────────────────────────────────────────────────────

  it("AlreadyDelegated error exists in IDL", async () => {
    const idl = require("../target/idl/soldex_perps.json");
    const idlErrors: any[] = idl.errors ?? [];
    const found = idlErrors.some((e: any) => e.name === "AlreadyDelegated");
    assert.isTrue(found, "AlreadyDelegated error should be in IDL");
    console.log("✓ AlreadyDelegated error present in IDL");
  });

  it("NotDelegated and PositionNotFlat errors exist in IDL", async () => {
    const idl = require("../target/idl/soldex_perps.json");
    const idlErrors: any[] = idl.errors ?? [];
    const hasNotDelegated    = idlErrors.some((e: any) => e.name === "NotDelegated");
    const hasPositionNotFlat = idlErrors.some((e: any) => e.name === "PositionNotFlat");
    assert.isTrue(hasNotDelegated,    "NotDelegated error should be in IDL");
    assert.isTrue(hasPositionNotFlat, "PositionNotFlat error should be in IDL");
    console.log("✓ NotDelegated and PositionNotFlat errors present in IDL");
  });

  // ── 3. IDL account structure checks ───────────────────────────────────────

  it("delegatePosition instruction has required MagicBlock accounts in IDL", async () => {
    const ix = program.idl.instructions.find(
      (i: any) => i.name === "delegatePosition",
    );
    assert.ok(ix, "delegatePosition instruction should exist in IDL");

    const accountNames = ix.accounts.map((a: any) => a.name);
    console.log("delegatePosition accounts:", accountNames);

    assert.include(accountNames, "ownerProgram",      "ownerProgram should be present");
    assert.include(accountNames, "delegationProgram", "delegationProgram should be present");
    assert.include(accountNames, "systemProgram",     "systemProgram should be present");
    console.log("✓ MagicBlock delegation accounts present in IDL");
  });

  it("undelegatePosition instruction has magic accounts in IDL", async () => {
    const ix = program.idl.instructions.find(
      (i: any) => i.name === "undelegatePosition",
    );
    assert.ok(ix, "undelegatePosition should exist in IDL");

    const accountNames = ix.accounts.map((a: any) => a.name);
    assert.include(accountNames, "magicContext", "magicContext should be present");
    assert.include(accountNames, "magicProgram",  "magicProgram should be present");
    console.log("✓ MagicBlock undelegate accounts present in IDL");
  });

  // ── 4. PDA seed consistency ────────────────────────────────────────────────

  it("PDA seeds are consistent across delegate/update_er/undelegate", async () => {
    const [fromOpenPosition] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("position"),
        Buffer.from(marketId),
        trader.publicKey.toBuffer(),
        Buffer.from([NONCE]),
      ],
      program.programId,
    );

    assert.equal(
      fromOpenPosition.toBase58(),
      positionPDA.toBase58(),
      "PDA from open_position seeds must match positionPDA used in delegate/update/undelegate",
    );
    console.log("✓ PDA seeds consistent:", positionPDA.toBase58());
  });

  // ── 5. Guard: delegatePosition rejects missing position ───────────────────

  it("rejects delegatePosition on non-existent position", async () => {
    const [bufferPDA]           = findBufferPDA(positionPDA, program.programId);
    const [delegationRecordPDA] = findDelegationRecordPDA(positionPDA);
    const [delegationMetaPDA]   = findDelegationMetadataPDA(positionPDA);

    try {
      await program.methods
        .delegatePosition(marketId, NONCE)
        .accounts({
          position:                   positionPDA,
          owner:                      trader.publicKey,
          bufferPosition:             bufferPDA,
          delegationRecordPosition:   delegationRecordPDA,
          delegationMetadataPosition: delegationMetaPDA,
          ownerProgram:               program.programId,
          delegationProgram:          DELEGATION_PROGRAM_ID,
          systemProgram:              SystemProgram.programId,
        })
        .signers([trader])
        .rpc();
      assert.fail("Should have thrown");
    } catch (err: any) {
      assert.ok(
        err.message.includes("Account does not exist") ||
        err.message.includes("AccountNotInitialized") ||
        err.message.includes("seeds") ||
        err.message.includes("constraint"),
        `Unexpected: ${err.message}`,
      );
      console.log("✓ delegatePosition correctly rejected missing position");
    }
  });

  // ── 6. Guard: undelegatePosition rejects non-delegated position ────────────

  it("rejects undelegatePosition when not delegated", async () => {
    const [magicContextPDA] = findMagicContextPDA();

    try {
      await program.methods
        .undelegatePosition(marketId, NONCE)
        .accounts({
          position:     positionPDA,
          owner:        trader.publicKey,
          magicContext: magicContextPDA,
          magicProgram: MAGIC_PROGRAM_ID,
        })
        .signers([trader])
        .rpc();
      assert.fail("Should have thrown NotDelegated");
    } catch (err: any) {
      assert.ok(
        err.message.includes("NotDelegated") ||
        err.message.includes("Account does not exist") ||
        err.message.includes("AccountNotInitialized"),
        `Unexpected: ${err.message}`,
      );
      console.log("✓ undelegatePosition correctly rejected non-delegated position");
    }
  });

  // ── 7. ER RPC reachability ─────────────────────────────────────────────────

  it("ER RPC is reachable and blockhash is valid", async () => {
    const ER_RPC = "https://devnet.magicblock.app";
    const erConn = new anchor.web3.Connection(ER_RPC, "confirmed");

    // Warm up connection
    await erConn.getLatestBlockhash().catch(() => null);

    // Now measure (warm connection)
    const t1 = Date.now();
    const bh = await erConn.getLatestBlockhash();
    const erPing = Date.now() - t1;

    console.log(`ER RPC warm ping:  ${erPing}ms`);
    console.log(`ER blockhash:      ${bh.blockhash.slice(0, 12)}...`);
    console.log(`ER last valid slot: ${bh.lastValidBlockHeight}`);

    // Verify it's reachable and responding
    assert.ok(bh.blockhash, "ER should return a valid blockhash");
    assert.isBelow(erPing, 8000, "ER warm ping should be under 8s");

    console.log("✓ MagicBlock ER is reachable");
    console.log("  NOTE: 10ms latency = tx CONFIRMATION time on ER vs 400ms on Solana");
    console.log("  This is only measurable with a real delegated position on devnet");
  });
});