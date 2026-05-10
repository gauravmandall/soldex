import { Connection, Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import pkg from "@coral-xyz/anchor";
const { Program, AnchorProvider, Wallet } = pkg;
import {
  TOKEN_PROGRAM_ID,
  getOrCreateAssociatedTokenAccount,
} from "@solana/spl-token";
import { readFileSync } from "fs";
import { homedir } from "os";

const RPC =
  "https://devnet.helius-rpc.com/?api-key=b3d57f82-1ea7-4424-ba3f-f93089499844";
const PROGRAM_ID = new PublicKey(
  "F8NaJJsJfScnUtz5mHvLfteU6i27n9zrS2dVWxaGvPHR",
);
const MARKET_ID_STR = "SOL-PERP";
// Old dead mint — we just need any ATA for it to dump tokens into
const OLD_MINT = new PublicKey("8cFwSBoxm4xHV9dr9Gp7tyQy4aG85q988yhjhMG6gULq");

const kp = JSON.parse(
  readFileSync(`${homedir()}/.config/solana/id.json`, "utf8"),
);
const authority = Keypair.fromSecretKey(new Uint8Array(kp));

const buf = Buffer.alloc(16);
Buffer.from(MARKET_ID_STR).copy(buf);
const marketId = Array.from(buf);

const [marketPDA, marketBump] = PublicKey.findProgramAddressSync(
  [Buffer.from("market"), buf],
  PROGRAM_ID,
);
const [vaultPDA] = PublicKey.findProgramAddressSync(
  [Buffer.from("vault"), buf],
  PROGRAM_ID,
);

console.log("Market PDA:", marketPDA.toString(), "bump:", marketBump);
console.log("Vault PDA:", vaultPDA.toString());
console.log("Authority:", authority.publicKey.toString());

const conn = new Connection(RPC, "confirmed");
const provider = new AnchorProvider(conn, new Wallet(authority), {});
const idl = JSON.parse(readFileSync("./target/idl/soldex_perps.json", "utf8"));
const program = new Program(idl, provider);

const vaultAcct = await conn.getAccountInfo(vaultPDA);
if (!vaultAcct) {
  console.log("✅ Vault doesn't exist — run init_market.mjs directly.");
  process.exit(0);
}

// Create or get an ATA for the old mint to dump tokens into
console.log("Creating destination ATA for old mint...");
const destATA = await getOrCreateAssociatedTokenAccount(
  conn,
  authority,
  OLD_MINT,
  authority.publicKey,
);
console.log("Destination ATA:", destATA.address.toString());

try {
  const sig = await program.methods
    .closeVault(marketId, marketBump)
    .accounts({
      authority: authority.publicKey,
      market: marketPDA,
      vault: vaultPDA,
      destination: destATA.address,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .signers([authority])
    .rpc();
  console.log("✅ Vault drained and closed! TX:", sig);
  console.log("Now run: node scripts/init_market.mjs");
} catch (e) {
  console.error("❌ Failed:", e.message);
}
