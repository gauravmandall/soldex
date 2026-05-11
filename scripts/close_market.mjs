import { Connection, Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import pkg from "@coral-xyz/anchor";
const { Program, AnchorProvider, Wallet, BN } = pkg;
import { readFileSync } from "fs";
import { homedir } from "os";

const RPC =
  "https://devnet.helius-rpc.com/?api-key=b3d57f82-1ea7-4424-ba3f-f93089499844";
const PROGRAM_ID = new PublicKey(
  "F8NaJJsJfScnUtz5mHvLfteU6i27n9zrS2dVWxaGvPHR",
);
const MARKET_ID_STR = "SOL-PERP"; // exactly 7 chars, padded to 16

const kp = JSON.parse(
  readFileSync(`${homedir()}/.config/solana/id.json`, "utf8"),
);
const authority = Keypair.fromSecretKey(new Uint8Array(kp));

const buf = Buffer.alloc(16);
Buffer.from(MARKET_ID_STR).copy(buf);
const marketId = Array.from(buf);

const [marketPDA] = PublicKey.findProgramAddressSync(
  [Buffer.from("market"), buf],
  PROGRAM_ID,
);

console.log("Market ID bytes:", MARKET_ID_STR);
console.log("Market PDA:", marketPDA.toString());
console.log("Authority:", authority.publicKey.toString());

const conn = new Connection(RPC, "confirmed");
const provider = new AnchorProvider(conn, new Wallet(authority), {});
const idl = JSON.parse(readFileSync("./target/idl/soldex_perps.json", "utf8"));
const program = new Program(idl, provider);

// Check if PDA exists first
const acct = await conn.getAccountInfo(marketPDA);
if (!acct) {
  console.log(
    "✅ Market PDA doesn't exist — nothing to close. Run init_market.mjs directly.",
  );
  process.exit(0);
}
console.log("Found dead PDA, closing...");

try {
  const sig = await program.methods
    .closeMarket(marketId)
    .accounts({
      authority: authority.publicKey,
      market: marketPDA,
      systemProgram: SystemProgram.programId,
    })
    .signers([authority])
    .rpc();
  console.log("✅ Dead PDA closed! TX:", sig);
} catch (e) {
  console.error("❌ Failed:", e.message);
}
