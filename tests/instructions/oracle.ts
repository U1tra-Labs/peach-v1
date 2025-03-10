import { program, provider } from "../helpers/setup";
import { PublicKey, Keypair } from "@solana/web3.js";
import { I80F48 } from "../helpers/I80F48";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";

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
