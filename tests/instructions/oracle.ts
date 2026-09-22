import { program, provider } from "../helpers/setup";
import { PublicKey, Keypair } from "@solana/web3.js";
import { I80F48 } from "../helpers/I80F48";
import { BN } from "@coral-xyz/anchor";

// Function to create a stub oracle
export async function createStubOracle(
  stubOracle: Keypair,
  marketPDA: PublicKey,
  mint: PublicKey,
  price: BN,
  owner: Keypair
) {
  const tx = await program.methods
    .stubOracleCreate(price)
    .accountsPartial({
      market: marketPDA,
      oracle: stubOracle.publicKey,
      admin: owner.publicKey,
      mint,
      payer: owner.publicKey,
    })
    .signers([stubOracle, owner])
    .rpc();

  return tx;
}
