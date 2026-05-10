import * as anchor from "@coral-xyz/anchor";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import fs from "fs";

// --- Config ---
const MARKET_PDA = new PublicKey(
  "Gd6265GNppu6qtNzfpPi7SiNYYXpXsEYssQ7Pp99eM48",
);
const DEVNET_RPC = "https://api.devnet.solana.com";
const WALLET_PATH = `${process.env.HOME}/.config/solana/id.json`;
const IDL_PATH = `${process.env.HOME}/soldex/target/idl/soldex_perps.json`;

// --- Setup ---
const connection = new Connection(DEVNET_RPC, "confirmed");
const rawKey = JSON.parse(fs.readFileSync(WALLET_PATH, "utf-8"));
const wallet = new anchor.Wallet(
  Keypair.fromSecretKey(Uint8Array.from(rawKey)),
);
const provider = new anchor.AnchorProvider(connection, wallet, {
  commitment: "confirmed",
});
anchor.setProvider(provider);

const idl = JSON.parse(fs.readFileSync(IDL_PATH, "utf-8"));
const program = new anchor.Program(idl, provider);

const marketId = Buffer.from("SOL-PERP");

async function setMaintenance(bps: number) {
  const isTest = bps === 9000;
  console.log(`Setting maintenanceMarginBps → ${bps}...`);
  const tx = await program.methods
    .updateMarket({
      marketId: Array.from(marketId),
      isActive: null,
      maxLeverageBps: null,
      makerFeeBps: null,
      takerFeeBps: null,
      initialMarginBps: isTest ? 10000 : 500,
      maintenanceMarginBps: bps,
    })
    .accounts({
      admin: provider.wallet.publicKey,
      market: MARKET_PDA,
    })
    .rpc();
  console.log(`✅ Done. Tx: ${tx}`);
}

async function verify() {
  const market = await (program.account as any).marketState.fetch(MARKET_PDA);
  console.log(`📊 maintenanceMarginBps: ${market.maintenanceMarginBps}`);
  console.log(`📊 initialMarginBps:     ${market.initialMarginBps}`);
}

const arg = process.argv[2]; // "set" or "revert" or "check"

if (arg === "set") {
  setMaintenance(9000).then(verify);
} else if (arg === "revert") {
  setMaintenance(250).then(verify);
} else if (arg === "check") {
  verify();
} else {
  console.log("Usage: npx ts-node update_market.ts [set|revert|check]");
}
