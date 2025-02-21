import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PeachV1 } from "../target/types/peach_v1";

describe("peach-v1", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env()
  anchor.setProvider(provider)

  const program = anchor.workspace.PeachV1 as Program<PeachV1>;

  const market_num = 4;
  const account_num = 1;

  const [marketPDA, _bump1] = anchor.web3.PublicKey.findProgramAddressSync(
        [
            Buffer.from("Market"),
            provider.wallet.publicKey.toBuffer(),
            new anchor.BN(market_num).toArrayLike(Buffer, "le", 4)
        ],
        program.programId
  );

    const [peachAccountPDA, _bump2] = anchor.web3.PublicKey.findProgramAddressSync(
        [
            Buffer.from("PeachAccount"),
            marketPDA.toBuffer(),
            provider.wallet.publicKey.toBuffer(),
            new anchor.BN(account_num).toArrayLike(Buffer, "le", 4)
        ],
        program.programId
  );

  it("Create market!", async () => {
    
    const tx = await program.methods
    .marketCreate(market_num, 0, 0)
    .accounts({
      market: marketPDA,
      creator: provider.wallet.publicKey,
      payer: provider.wallet.publicKey,
      systemProgram: anchor.web3.SystemProgram.programId,
      rent: anchor.web3.SYSVAR_RENT_PUBKEY,
    })
    .rpc();
    const market = await program.account.market.fetch(marketPDA);
    console.log("Your transaction signature", tx);
    console.log("Market", market);
  });

  it("Create peach account!", async () => { 
    const token_count = 0;
    const name = "test";

    const tx = await program.methods.
      accountCreate(account_num, token_count, name)
      .accounts({
        market: marketPDA,
        account: peachAccountPDA,
        owner: provider.wallet.publicKey,
        payer: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId
      })
      .rpc();
    const account = await program.account.peachAccountFixed.fetch(peachAccountPDA);
    console.log("Your transaction signature", tx);
    console.log("Peach account", account);
  
  });
});

