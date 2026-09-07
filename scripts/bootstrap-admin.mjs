import fs from "node:fs";
import { Connection, Keypair, PublicKey, Transaction, TransactionInstruction } from "../app/node_modules/@solana/web3.js/lib/index.cjs.js";

const PROGRAM_ID = new PublicKey("4cJDBQmPnuf3GZrP9SW17wDhPpBVcvfNk51MiMrUCCfQ");
const TARGET_ADMIN = new PublicKey("Dy6mBH4YeqJCRZohd39iSFaf4jyLaxPeBakbZwt1jToL");
const keypairPath = process.env.SOLANA_KEYPAIR;
if (!keypairPath) throw new Error("Set SOLANA_KEYPAIR to the CLI keypair path");
const payer = Keypair.fromSecretKey(Uint8Array.from(JSON.parse(fs.readFileSync(keypairPath, "utf8"))));
const connection = new Connection(process.env.SOLANA_RPC ?? "https://api.devnet.solana.com", "confirmed");
const [config] = PublicKey.findProgramAddressSync([Buffer.from("config")], PROGRAM_ID);
const before = await connection.getAccountInfo(config);
if (!before) {
  const init = new TransactionInstruction({ programId: PROGRAM_ID, data: Buffer.from([0]), keys: [{pubkey:payer.publicKey,isSigner:true,isWritable:true},{pubkey:config,isSigner:false,isWritable:true}] });
  const initSignature = await connection.sendTransaction(new Transaction().add(init), [payer]);
  await connection.confirmTransaction(initSignature, "confirmed");
  console.log("initialize", initSignature);
}
const propose = new TransactionInstruction({ programId: PROGRAM_ID, data: Buffer.concat([Buffer.from([10]),TARGET_ADMIN.toBuffer()]), keys: [{pubkey:payer.publicKey,isSigner:true,isWritable:false},{pubkey:config,isSigner:false,isWritable:true}] });
const signature = await connection.sendTransaction(new Transaction().add(propose), [payer]);
await connection.confirmTransaction(signature, "confirmed");
const data = (await connection.getAccountInfo(config))?.data;
if (!data) throw new Error("Config readback failed");
console.log(JSON.stringify({config:config.toBase58(),currentAdmin:new PublicKey(data.subarray(8,40)).toBase58(),pendingAdmin:new PublicKey(data.subarray(40,72)).toBase58(),proposeSignature:signature}));
