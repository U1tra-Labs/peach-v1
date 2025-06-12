import { program, provider } from "../helpers/setup";
import { PublicKey, Keypair } from "@solana/web3.js";
import { I80F48 } from "../helpers/I80F48";
import { BN } from "@coral-xyz/anchor";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";

// Function to create a stub oracle
export async function createStubOracle(stubOracle: Keypair, marketPDA: PublicKey, mint: PublicKey, price: BN, owner: Keypair) {  
  const tx = await program.methods
    .stubOracleCreate(price)
    .accounts({
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

export async function closeStubOracle(oracle: Keypair, market: PublicKey, admin: Keypair, programWallet: Keypair) {
  const tx = await program.methods.stubOracleClose()
    .accounts({
        market: market,
        admin: admin.publicKey,
        oracle: oracle.publicKey,
        solDestination: programWallet.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      }
    )
    .signers([admin])
    .rpc();
  
  return tx;
}
