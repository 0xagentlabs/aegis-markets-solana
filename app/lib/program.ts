import { PublicKey, TransactionInstruction } from "@solana/web3.js";

export const PROGRAM_ID = new PublicKey(process.env.NEXT_PUBLIC_PROGRAM_ID ?? "4cJDBQmPnuf3GZrP9SW17wDhPpBVcvfNk51MiMrUCCfQ");
export const ADMIN = new PublicKey("Dy6mBH4YeqJCRZohd39iSFaf4jyLaxPeBakbZwt1jToL");
export const TOKEN_PROGRAM = new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
export const configPda = () => PublicKey.findProgramAddressSync([Buffer.from("config")], PROGRAM_ID)[0];
export const reservePda = (mint: PublicKey) => PublicKey.findProgramAddressSync([Buffer.from("reserve"), mint.toBuffer()], PROGRAM_ID)[0];
export const positionPda = (owner: PublicKey) => PublicKey.findProgramAddressSync([Buffer.from("position"), owner.toBuffer()], PROGRAM_ID)[0];

const amountData = (tag: number, amount: bigint) => { const d = Buffer.alloc(9); d[0] = tag; d.writeBigUInt64LE(amount, 1); return d; };
export function userInstruction(tag: 4 | 5 | 6 | 7, amount: bigint, accounts: { user: PublicKey; reserve: PublicKey; position: PublicKey; userToken: PublicKey; mint: PublicKey; vault: PublicKey; oracle: PublicKey }) {
  return new TransactionInstruction({ programId: PROGRAM_ID, data: amountData(tag, amount), keys: [
    { pubkey: accounts.user, isSigner: true, isWritable: true }, { pubkey: accounts.reserve, isSigner: false, isWritable: true },
    { pubkey: accounts.position, isSigner: false, isWritable: true }, { pubkey: accounts.userToken, isSigner: false, isWritable: true },
    { pubkey: accounts.mint, isSigner: false, isWritable: false }, { pubkey: accounts.vault, isSigner: false, isWritable: true },
    { pubkey: accounts.oracle, isSigner: false, isWritable: false }, { pubkey: TOKEN_PROGRAM, isSigner: false, isWritable: false }
  ] });
}
export function initializePosition(user: PublicKey) { return new TransactionInstruction({ programId: PROGRAM_ID, data: Buffer.from([3]), keys: [{pubkey:user,isSigner:true,isWritable:true},{pubkey:positionPda(user),isSigner:false,isWritable:true}] }); }
export function initializeConfig(admin: PublicKey) { return new TransactionInstruction({ programId: PROGRAM_ID, data: Buffer.from([0]), keys: [{pubkey:admin,isSigner:true,isWritable:true},{pubkey:configPda(),isSigner:false,isWritable:true}] }); }
export function initializeReserve(admin: PublicKey, mint: PublicKey, feedIdHex: string, decimals: number) {
  const feed = Buffer.from(feedIdHex.replace(/^0x/, ""), "hex"); if (feed.length !== 32) throw new Error("Pyth feed ID 必须是 32 字节 hex");
  const data=Buffer.alloc(100); data[0]=1; mint.toBuffer().copy(data,1); feed.copy(data,33); data[65]=decimals;
  data.writeUInt16LE(7500,66); data.writeUInt16LE(8000,68); data.writeUInt16LE(500,70); data.writeUInt16LE(1000,72);
  data.writeBigUInt64LE(1_000_000_000_000n,74); data.writeBigUInt64LE(800_000_000_000n,82); data.writeBigUInt64LE(60n,90); data.writeUInt16LE(100,98);
  return new TransactionInstruction({programId:PROGRAM_ID,data,keys:[{pubkey:admin,isSigner:true,isWritable:true},{pubkey:configPda(),isSigner:false,isWritable:false},{pubkey:reservePda(mint),isSigner:false,isWritable:true}]});
}
export function pauseReserve(admin: PublicKey, reserve: PublicKey, paused: boolean) { return new TransactionInstruction({ programId: PROGRAM_ID, data: Buffer.from([9, paused ? 1 : 0]), keys: [{pubkey:admin,isSigner:true,isWritable:true},{pubkey:configPda(),isSigner:false,isWritable:false},{pubkey:reserve,isSigner:false,isWritable:true}] }); }
export function proposeAdmin(admin: PublicKey, pending: PublicKey) { return new TransactionInstruction({programId:PROGRAM_ID,data:Buffer.concat([Buffer.from([10]),pending.toBuffer()]),keys:[{pubkey:admin,isSigner:true,isWritable:false},{pubkey:configPda(),isSigner:false,isWritable:true}]}); }
export function acceptAdmin(pending: PublicKey) { return new TransactionInstruction({programId:PROGRAM_ID,data:Buffer.from([11]),keys:[{pubkey:pending,isSigner:true,isWritable:false},{pubkey:configPda(),isSigner:false,isWritable:true}]}); }
