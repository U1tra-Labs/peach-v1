import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";
import { Connection, Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { provider, createTokenMint, derivePDAs, envProviderPayer } from "./helpers/setup";
import { createMarket, createPeachAccount, createStubOracle } from "./helpers/transactions";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";

// Test suite
describe("peach-v1", () => {
  const program = anchor.workspace.PeachV1;
  const stubOracle = Keypair.generate();
  let marketPDA: PublicKey, peachAccountPDA: PublicKey, bankPDA: PublicKey, vaultPDA: PublicKey, mintInfoPDA: PublicKey;
  let mint: PublicKey;

  // Update these values as needed
  let marketNum = 7, accountNum = 0, tokenIndex = 3, price = 1.0;

  before(async () => {
    // Create mint
    mint = await createTokenMint(10);

    // Derive PDAs
    ({ marketPDA, peachAccountPDA, bankPDA, vaultPDA, mintInfoPDA } = derivePDAs(marketNum, accountNum, tokenIndex, mint));

    console.log("Market PDA:", marketPDA.toBase58());
    console.log("Peach Account PDA:", peachAccountPDA.toBase58());
    console.log("Bank PDA:", bankPDA.toBase58());
    console.log("Vault PDA:", vaultPDA.toBase58());
    console.log("Mint Info PDA:", mintInfoPDA.toBase58());
  });

  it("Creates a market", async () => {
    const tx = await createMarket(marketPDA, marketNum);
    console.log("Market Created: ", tx);

    const market = await program.account.market.fetch(marketPDA);
    assert.equal(market.marketNum, marketNum);
  });

  it("Creates a peach account", async () => {
    const tx = await createPeachAccount(peachAccountPDA, marketPDA, accountNum);
    console.log("Peach Account Created: ", tx);

    const account = await program.account.peachAccountFixed.fetch(peachAccountPDA);
    assert.equal(account.accountNum, accountNum);
  });

  it("Creates a stub oracle", async () => {
    const tx = await createStubOracle(stubOracle, marketPDA, mint, price);
    console.log("Stub Oracle Created: ", tx);

    const oracle = await program.account.stubOracle.fetch(stubOracle.publicKey);
    assert.ok(oracle.mint.equals(mint));
  });

  it("Registers a token", async () => {
    const ix1 = await program.methods.tokenVaultCreate(tokenIndex)
      .accounts({
        market: marketPDA,
        admin: provider.wallet.publicKey,
        mint,
        vault: vaultPDA,
        mintInfo: mintInfoPDA,
        payer: provider.wallet.publicKey, // This is the payer
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .instruction();


    const ix2 = await program.methods.tokenRegister(
      tokenIndex,
      "USDC",
      {
        confFilter: 0.01,
        maxStalenessSlots: 60,
      },
      {
        util0: 0.8,
        rate0: 0.02,
        util1: 0.9,
        rate1: 0.05,
        maxRate: 0.1,
        adjustmentFactor: 0.01,
      },
      0.002,
      0.001,
      0.8,
      0.7,
      1.2,
      1.3,
      0.05,
      60,
      0.02,
      0.03,
      0.1,
      new anchor.BN(600),
      new anchor.BN(1000000),
      50000,
      50000,
      0,
      0.5,
      0.8,
      true,
      new anchor.BN(1000000),
      0.01,
      0.02,
      false,
      0.001,
      "tier",
    )
      .accounts({
        market: marketPDA,
        admin: provider.wallet.publicKey,
        mint,
        bank: bankPDA,
        vault: vaultPDA,
        mintInfo: mintInfoPDA,
        oracle: stubOracle.publicKey,
        fallbackOracle: stubOracle.publicKey,
        payer: provider.wallet.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      }).instruction();

    const tx = new anchor.web3.Transaction();
    tx.add(ix1);
    tx.add(ix2);

    const connection = new Connection("https://api.devnet.solana.com", "confirmed");

    const sig = await anchor.web3.sendAndConfirmTransaction(
      connection, tx, [envProviderPayer]
    );


    console.log("Token registered: ", sig);

    const mintInfoAccount = await program.account.mintInfo.fetch(mintInfoPDA);
    const bankAccount = await program.account.bank.fetch(bankPDA);
    console.log("Mint Info Account: ", mintInfoAccount);
    console.log("Bank Account: ", bankAccount);
    assert.ok(mintInfoAccount.mint.equals(mint));
    assert.ok(mintInfoAccount.oracle.equals(stubOracle.publicKey));
  });

});

