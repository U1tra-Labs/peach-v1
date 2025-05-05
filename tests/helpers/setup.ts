import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PeachV1 } from "../../target/types/peach_v1";
import { Keypair, PublicKey } from "@solana/web3.js";
import { createAssociatedTokenAccount, createMint, getOrCreateAssociatedTokenAccount, mintTo, transfer } from "@solana/spl-token";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";

export const provider = anchor.AnchorProvider.env();
anchor.setProvider(provider);
export const program = anchor.workspace.PeachV1 as Program<PeachV1>;
export const envProviderPayer = (provider.wallet as NodeWallet).payer;

// Constants
const MARKET_SEED = "Market";
const PEACH_ACCOUNT_SEED = "PeachAccount";
const BANK_SEED = "Bank";
const VAULT_SEED = "Vault";
const MINT_INFO_SEED = "MintInfo";

// Function to create a mint
export async function createTokenMint(decimals = 10, owner: Keypair): Promise<PublicKey> {
  return await createMint(
    program.provider.connection,
    owner,
    owner.publicKey,
    owner.publicKey,
    decimals
  );
}

export async function createTokenAccount(mint: PublicKey, owner: Keypair): Promise<PublicKey> {
  const getAssociatedTokenAddress = await createAssociatedTokenAccount(
    program.provider.connection,
    owner,
    mint,
    owner.publicKey,
  );

  // const transactionSignature = await mintTo(
  //   program.provider.connection,
  //   owner,
  //   mint,
  //   getAssociatedTokenAddress,
  //   owner,
  //   2000000,
  //   [owner]
  // );

  return getAssociatedTokenAddress;
}

export async function transferToken(mint: PublicKey, sender_ata: PublicKey, sender: Keypair, receiver: Keypair, amount: number) {
  // const senderAssociatedTokenAddress = await getOrCreateAssociatedTokenAccount(
  //   program.provider.connection,
  //   sender,
  //   mint,
  //   sender.publicKey,
  // );

  const receipientAssociatedTokenAddress = await createAssociatedTokenAccount(
    program.provider.connection,
    receiver,
    mint,
    receiver.publicKey,
  );
  
  const transactionSignature = await transfer(
    program.provider.connection,
    sender,
    sender_ata,
    receipientAssociatedTokenAddress,
    sender,
    amount,
  );

  return receipientAssociatedTokenAddress;
}

// Function to derive market PDA
export function deriveMarketPDA(marketNum: number, owner: PublicKey): PublicKey {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from(MARKET_SEED), owner.toBuffer(), new anchor.BN(marketNum).toArrayLike(Buffer, "le", 4)],
    program.programId
  )[0];
}

// Function to derive peach account PDA
export function derivePeachAccountPDA(market: PublicKey, accountNum: number, user: PublicKey): PublicKey {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from(PEACH_ACCOUNT_SEED), market.toBuffer(), user.toBuffer(), new anchor.BN(accountNum).toArrayLike(Buffer, "le", 4)],
    program.programId
  )[0];
}

// Function to derive peach account PDA
export function testDerivePeachAccountPDA(market: PublicKey, accountNum: number, user: PublicKey): PublicKey {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from(PEACH_ACCOUNT_SEED), market.toBuffer(), user.toBuffer(), new anchor.BN(accountNum).toArrayLike(Buffer, "le", 4)],
    program.programId
  )[0];
}

// Function to derive bank PDA
export function deriveBankPDA(market: PublicKey, tokenIndex: number): PublicKey {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from(BANK_SEED), market.toBuffer(), new anchor.BN(tokenIndex).toArrayLike(Buffer, "le", 2), new anchor.BN(0).toArrayLike(Buffer, "le", 4)],
    program.programId
  )[0];
}

// Function to derive vault PDA
export function deriveVaultPDA(market: PublicKey, tokenIndex: number): PublicKey {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from(VAULT_SEED), market.toBuffer(), new anchor.BN(tokenIndex).toArrayLike(Buffer, "le", 2), new anchor.BN(0).toArrayLike(Buffer, "le", 4)],
    program.programId
  )[0];
}

// Function to derive mint info PDA
export function deriveMintInfoPDA(market: PublicKey, mint: PublicKey): PublicKey {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from(MINT_INFO_SEED), market.toBuffer(), mint.toBuffer()],
    program.programId
  )[0];
}