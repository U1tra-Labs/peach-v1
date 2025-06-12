import { program, provider } from "../helpers/setup";
import { PublicKey, Keypair } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { User } from "../objects/user";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";

// Function to create a peach account
export async function createPeachAccount(user: User, market: PublicKey) {
  const token_count = 4;
  const name = "test";

  const tx = await program.methods
    .accountCreate(user.accountNum, token_count, name)
    .accounts({
      market: market,
      peachAccount: user.peachAccount,
      owner: user.wallet.publicKey,
      payer: user.wallet.publicKey,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
    .signers([user.wallet])
    .rpc();

  return tx;
}

export async function closePeachAccount(user: User, market: PublicKey) {
  const tx =  await program.methods.accountClose(true)
  .accounts({
    market: market,
    account: user.peachAccount,
    owner: user.wallet.publicKey,
    solDestination: user.wallet.publicKey,
    tokenProgram: TOKEN_PROGRAM_ID,
  }
  )
  .signers([user.wallet])
    .rpc();
  
  return tx;
}