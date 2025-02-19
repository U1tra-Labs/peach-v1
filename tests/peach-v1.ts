import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PeachV1 } from "../target/types/peach_v1";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";

describe("peach-v1", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env()
  anchor.setProvider(provider)

  const program = anchor.workspace.PeachV1 as Program<PeachV1>;

  const market_num = 1;

  const [marketPDA, _bump1] = anchor.web3.PublicKey.findProgramAddressSync(
        [
            Buffer.from("Market"),
            provider.wallet.publicKey.toBuffer(),
            new anchor.BN(market_num).toArrayLike(Buffer, "le", 4)
        ],
        program.programId
  );

  const [reserveVaultPDA, _bump2] = anchor.web3.PublicKey.findProgramAddressSync(
        [
            Buffer.from("ReserveVault"),
            marketPDA.toBuffer(),
        ],
    program.programId
  );

  // USDC Reserve Mint
  const reserveMint = new anchor.web3.PublicKey("BVqRM5tbqerXFnGBSDZMvGEvdUx3aVNUadBiQBbjAjxB");

  it("Create market!", async () => {
    
    const tx = await program.methods
    .marketCreate(market_num, 0, 0)
    .accounts({
      market: marketPDA,
      creator: provider.wallet.publicKey,
      reserveMint: reserveMint,
      reserveVault: reserveVaultPDA,
      payer: provider.wallet.publicKey,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: anchor.web3.SystemProgram.programId,
      rent: anchor.web3.SYSVAR_RENT_PUBKEY,
    })
    .rpc();
    const market = await program.account.market.fetch(marketPDA);
    console.log("Your transaction signature", tx);
    console.log("Market", market);
  });
});

