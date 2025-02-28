import { program, provider } from "./setup";
import { PublicKey, Keypair } from "@solana/web3.js";
import { I80F48 } from "./I80F48";
import * as anchor from "@coral-xyz/anchor";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";

// Function to create a market
export async function createMarket(marketPDA: PublicKey, marketNum: number, owner: Keypair) {
  const tx = await program.methods
    .marketCreate(marketNum, 0, 0)
    .accounts({
      market: marketPDA,
      creator: owner.publicKey,
      payer: owner.publicKey,
      systemProgram: anchor.web3.SystemProgram.programId,
      rent: anchor.web3.SYSVAR_RENT_PUBKEY,
    })
    .signers([owner])
    .rpc();

  return tx;
}

// Function to create a peach account
export async function createPeachAccount(peachAccountPDA: PublicKey, marketPDA: PublicKey, accountNum: number) {
  const token_count = 2;
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
    .signers([(provider.wallet as NodeWallet).payer])
    .rpc();

  return tx;
}

// Function to create a stub oracle
export async function createStubOracle(stubOracle: Keypair, marketPDA: PublicKey, mint: PublicKey, price: number, owner: Keypair) {
  const tx = await program.methods
    .stubOracleCreate({ val: I80F48.fromNumber(price).getData() })
    .accounts({
      market: marketPDA,
      admin: owner.publicKey,
      oracle: stubOracle.publicKey,
      mint,
      payer: owner.publicKey,
    })
    .signers([stubOracle, owner])
    .rpc();

  return tx;
}


