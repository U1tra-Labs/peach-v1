import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PeachV1 } from "../../target/types/peach_v1";
import { Keypair, LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";
import { createAccount, createAssociatedTokenAccount, createMint, getOrCreateAssociatedTokenAccount, mintTo, transfer } from "@solana/spl-token";
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

export async function getFundedWallet() {
  const wallet = new Keypair();
  const signature = await provider.connection.requestAirdrop(wallet.publicKey, 2 * LAMPORTS_PER_SOL);
  await provider.connection.confirmTransaction(signature);
  console.log("Funded wallet:", wallet.publicKey.toBase58());
  console.log("Signature:", signature);
  return wallet;
}

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

  const transactionSignature = await mintTo(
    program.provider.connection,
    owner,
    mint,
    getAssociatedTokenAddress,
    owner,
    2000000,
    [owner]
  );

  return getAssociatedTokenAddress;
}

export async function transferToken(mint: PublicKey, owner: Keypair, amount: number) {
  const senderAssociatedTokenAddress = await getOrCreateAssociatedTokenAccount(
    program.provider.connection,
    owner,
    mint,
    owner.publicKey,
  ); 

  const receipientAssociatedTokenAddress = await createAssociatedTokenAccount(
    program.provider.connection,
    envProviderPayer,
    mint,
    provider.wallet.publicKey,
  ); 

  const transactionSignature = await transfer(
    program.provider.connection,
    owner,
    senderAssociatedTokenAddress.address,
    receipientAssociatedTokenAddress,
    owner,
    amount,
  );   
}

// Function to derive PDAs
export function derivePDAs(marketNum: number, accountNum: number, tokenIndex: number, mint: PublicKey, owner: PublicKey) {
  const marketPDA = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from(MARKET_SEED), owner.toBuffer(), new anchor.BN(marketNum).toArrayLike(Buffer, "le", 4)],
    program.programId
  )[0];

  const peachAccountPDA = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from(PEACH_ACCOUNT_SEED), marketPDA.toBuffer(), provider.wallet.publicKey.toBuffer(), new anchor.BN(accountNum).toArrayLike(Buffer, "le", 4)],
    program.programId
  )[0];

  const bankPDA = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from(BANK_SEED), marketPDA.toBuffer(), new anchor.BN(tokenIndex).toArrayLike(Buffer, "le", 2), new anchor.BN(0).toArrayLike(Buffer, "le", 4)],
    program.programId
  )[0];

  const vaultPDA = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from(VAULT_SEED), marketPDA.toBuffer(), new anchor.BN(tokenIndex).toArrayLike(Buffer, "le", 2), new anchor.BN(0).toArrayLike(Buffer, "le", 4)],
    program.programId
  )[0];

  const mintInfoPDA = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from(MINT_INFO_SEED), marketPDA.toBuffer(), mint.toBuffer()],
    program.programId
  )[0];

  return { marketPDA, peachAccountPDA, bankPDA, vaultPDA, mintInfoPDA };
}