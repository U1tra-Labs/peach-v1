import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PeachV1 } from "../../target/types/peach_v1";
import { Keypair, PublicKey, sendAndConfirmTransaction, Transaction } from "@solana/web3.js";
import { ASSOCIATED_TOKEN_PROGRAM_ID, createAssociatedTokenAccount, createAssociatedTokenAccountInstruction, createMint, createTransferCheckedInstruction, getAssociatedTokenAddress, getAssociatedTokenAddressSync, getOrCreateAssociatedTokenAccount, mintTo, TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID, transfer } from "@solana/spl-token";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";
import { USDC_MINT_MAINNET } from "./const";

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

export async function getUSDCMint(): Promise<PublicKey> {
  if (program.provider.connection.rpcEndpoint.includes("mainnet") || program.provider.connection.rpcEndpoint.includes("staked")) {
      return USDC_MINT_MAINNET;
  }
  return await createTokenMint(6, envProviderPayer);
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

export async function createTokenAccount(mint: PublicKey, owner: Keypair, programId: PublicKey): Promise<PublicKey> {
  try {
    let tokenATA = await getOrCreateAssociatedTokenAccount(
      program.provider.connection,
      owner, 
      mint,
      owner.publicKey,
      false, // allowOwnerOffCurve
      undefined, // commitment
      undefined, // confirmOptions
      programId,
      ASSOCIATED_TOKEN_PROGRAM_ID // associatedTokenProgramId 
    );
    // console.log("Token Account Address:", tokenATA);
    
    if (program.provider.connection.rpcEndpoint.includes("mainnet")) {
      // console.log("Mainnet detected, no minting tokens.");
      return tokenATA.address;
    }

    // console.log("Minting tokens...");

    const transactionSignature = await mintTo(
      program.provider.connection,
      owner,
      mint,
      tokenATA.address,
      owner,
      100 * 10 ** 6,
      [owner]
    );

    return tokenATA.address;
  }
  catch (error) {
      console.error("Error creating associated token account:", error);
  }
}

export async function transferToken(programId: PublicKey, mint: PublicKey, sender_ata: PublicKey, sender: Keypair, receiver: Keypair, amount: number) {
  
  let receipientATA = await getOrCreateAssociatedTokenAccount(
    program.provider.connection,
    receiver, 
    mint,
    receiver.publicKey,
    false, // allowOwnerOffCurve
    undefined, // commitment
    undefined, // confirmOptions
    programId,
    ASSOCIATED_TOKEN_PROGRAM_ID // associatedTokenProgramId 
  );

  const balance = await program.provider.connection.getTokenAccountBalance(receipientATA.address);

  if (balance.value.uiAmount < amount / 10 ** 6) {
    const tx = new Transaction().add(
      createTransferCheckedInstruction(
        sender_ata,
        mint,
        receipientATA.address,
        sender.publicKey,
        amount,
        6, // decimals
        [],
        programId
      )
    );

    const signature = await sendAndConfirmTransaction(program.provider.connection, tx, [sender], {
      commitment: "confirmed"
    });

    console.log(`Transfered tokens to ${receiver.publicKey}: ${signature}`);
  }

  return receipientATA.address;
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
export function deriveBankPDA(market: PublicKey, tokenIndex: number, bankNumber: number): PublicKey {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from(BANK_SEED), market.toBuffer(), new anchor.BN(tokenIndex).toArrayLike(Buffer, "le", 2), new anchor.BN(bankNumber).toArrayLike(Buffer, "le", 4)],
    program.programId
  )[0];
}

// Function to derive vault PDA
export function deriveVaultPDA(market: PublicKey, tokenIndex: number, vaultNumber: number): PublicKey {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from(VAULT_SEED), market.toBuffer(), new anchor.BN(tokenIndex).toArrayLike(Buffer, "le", 2), new anchor.BN(vaultNumber).toArrayLike(Buffer, "le", 4)],
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