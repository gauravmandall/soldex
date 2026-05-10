import { Connection, Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import pkg from "@coral-xyz/anchor";
const { Program, AnchorProvider, Wallet, BN } = pkg;
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { readFileSync } from "fs";
import { homedir } from "os";

const RPC =
  "https://devnet.helius-rpc.com/?api-key=b3d57f82-1ea7-4424-ba3f-f93089499844";
const PROGRAM_ID = new PublicKey(
  "F8NaJJsJfScnUtz5mHvLfteU6i27n9zrS2dVWxaGvPHR",
);
const QUOTE_MINT = new PublicKey(
  "6kgSZ26MzBqHQBZf6f6HxRwFLMFZzXgDj2j4vs11MTRB",
);
const PYTH_FEED = new PublicKey("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE");
const MARKET_ID_STR = "SOL-PERP";

const kp = JSON.parse(
  readFileSync(`${homedir()}/.config/solana/id.json`, "utf8"),
);
const admin = Keypair.fromSecretKey(new Uint8Array(kp));

const _buf = Buffer.alloc(16);
Buffer.from(MARKET_ID_STR).copy(_buf);
const marketId = Array.from(_buf);

const [marketPDA] = PublicKey.findProgramAddressSync(
  [Buffer.from("market"), Buffer.from(marketId)],
  PROGRAM_ID,
);
const [vaultPDA] = PublicKey.findProgramAddressSync(
  [Buffer.from("vault"), Buffer.from(marketId)],
  PROGRAM_ID,
);

console.log("Market ID:  ", MARKET_ID_STR);
console.log("Market PDA: ", marketPDA.toString());
console.log("Vault PDA:  ", vaultPDA.toString());
console.log("Admin:      ", admin.publicKey.toString());

const conn = new Connection(RPC, "confirmed");
const provider = new AnchorProvider(conn, new Wallet(admin), {});
const idl = JSON.parse(readFileSync("./target/idl/soldex_perps.json", "utf8"));
const program = new Program(idl, provider);

try {
  const sig = await program.methods
    .initializeMarket({
      marketId,
      priceFeed: PYTH_FEED,
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
      quoteMint: QUOTE_MINT,
      vault: vaultPDA,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .signers([admin])
    .rpc();

  console.log("\n✅ Market initialized!");
  console.log("   TX:", sig);
  console.log("\n📋 Use this in ws_test.mjs:");
  console.log(`   MARKET_ID=${MARKET_ID_STR} node ws_test.mjs deposit`);
} catch (e) {
  console.error("❌ Failed:", e.message);
}
