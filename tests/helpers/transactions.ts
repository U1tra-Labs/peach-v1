import { program, provider } from "./setup";
import { PublicKey, Keypair } from "@solana/web3.js";
import { I80F48 } from "./I80F48";
import * as anchor from "@coral-xyz/anchor";

// Function to create a market
export async function createMarket(marketPDA: PublicKey, marketNum: number) {
  const tx = await program.methods
    .marketCreate(marketNum, 0, 0)
    .accounts({
      market: marketPDA,
      creator: provider.wallet.publicKey,
      payer: provider.wallet.publicKey,
      systemProgram: anchor.web3.SystemProgram.programId,
      rent: anchor.web3.SYSVAR_RENT_PUBKEY,
    })
    .rpc();

  return tx;
}

// Function to create a peach account
export async function createPeachAccount(peachAccountPDA: PublicKey, marketPDA: PublicKey, accountNum: number) {
  const token_count = 0;
  const name = "test";

  const tx = await program.methods
    .accountCreate(accountNum, token_count, name)
    .accounts({
      market: marketPDA,
      account: peachAccountPDA,
      owner: provider.wallet.publicKey,
      payer: provider.wallet.publicKey,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
    .rpc();

  return tx;
}

// Function to create a stub oracle
export async function createStubOracle(stubOracle: Keypair, marketPDA: PublicKey, mint: PublicKey, price: number) {
  const tx = await program.methods
    .stubOracleCreate({ val: I80F48.fromNumber(price).getData() })
    .accounts({
      group: marketPDA,
      admin: provider.wallet.publicKey,
      oracle: stubOracle.publicKey,
      mint,
      payer: provider.wallet.publicKey,
    })
    .signers([stubOracle])
    .rpc();

  return tx;
}
