import { program, provider } from "../helpers/setup";
import { PublicKey, Keypair } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";

// Function to create a peach account
export async function createPeachAccount(peachAccount: PublicKey, market: PublicKey, accountNum: number, user: Keypair) {
  const token_count = 2;
  const name = "test";

  const tx = await program.methods
    .accountCreate(accountNum, token_count, name)
    .accounts({
      market: market,
      account: peachAccount,
      owner: user.publicKey,
      payer: user.publicKey,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
    .signers([user])
    .rpc();

  return tx;
}