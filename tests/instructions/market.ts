import { envProviderPayer, program } from "../helpers/setup";
import { PublicKey, Keypair } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";

// Function to create a market
export async function marketCreate(
  marketPDA: PublicKey,
  marketNum: number,
  owner: Keypair
) {
  const tx = await program.methods
    .marketCreate(marketNum, 1, 0)
    .accountsPartial({
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

export async function marketClose(market: PublicKey, admin: Keypair) {
  const tx = await program.methods
    .marketClose()
    .accountsPartial({
      market,
      admin: admin.publicKey,
      solDestination: envProviderPayer.publicKey,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .signers([admin])
    .rpc();

  return tx;
}
