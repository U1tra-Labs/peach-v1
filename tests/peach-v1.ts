import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";
import { Connection, Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { provider, createTokenMint, derivePDAs, envProviderPayer, createTokenAccount, getFundedWallet, transferToken } from "./helpers/setup";
import { createMarket, createPeachAccount, createStubOracle } from "./helpers/transactions";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";

// Test suite
describe("peach-v1", () => {
  const program = anchor.workspace.PeachV1;
  const stubOracle = Keypair.generate();
  let owner: Keypair;
  let marketPDA: PublicKey, peachAccountPDA: PublicKey, bankPDA: PublicKey, vaultPDA: PublicKey, mintInfoPDA: PublicKey;
  let mint: PublicKey;
  let tokenAccount: PublicKey;

  // Update these values as needed
  let marketNum = 3, accountNum = 0, tokenIndex = 1, price = 1.0;
  let deposit_amount = new anchor.BN(100), withdraw_amount = new anchor.BN(5);

  before(async () => {
    owner = await getFundedWallet();

    // Create mint
    mint = await createTokenMint(10, owner);
    console.log("Mint created: ", mint.toBase58());
    tokenAccount = await createTokenAccount(mint, owner);
    transferToken(mint, owner, 100);

    // Derive PDAs
    ({ marketPDA, peachAccountPDA, bankPDA, vaultPDA, mintInfoPDA } = derivePDAs(marketNum, accountNum, tokenIndex, mint, owner.publicKey));

    console.log("Market PDA:", marketPDA.toBase58());
    console.log("Peach Account PDA:", peachAccountPDA.toBase58());
    console.log("Bank PDA:", bankPDA.toBase58());
    console.log("Vault PDA:", vaultPDA.toBase58());
    console.log("Mint Info PDA:", mintInfoPDA.toBase58());
  });

  it("Creates a market", async () => {
    const tx = await createMarket(marketPDA, marketNum, owner);
    console.log("Market Created: ", tx);

    const market = await program.account.market.fetch(marketPDA);
    assert.equal(market.marketNum, marketNum);
  });

  it("Creates a peach account", async () => {
    const tx = await createPeachAccount(peachAccountPDA, marketPDA, accountNum);
    console.log("Peach Account Created: ", tx);

    const account = await program.account.peachAccount.fetch(peachAccountPDA);
    assert.equal(account.accountNum, accountNum);
  });

  it("Creates a stub oracle", async () => {
    const tx = await createStubOracle(stubOracle, marketPDA, mint, price, owner);
    console.log("Stub Oracle Created: ", tx);

    const oracle = await program.account.stubOracle.fetch(stubOracle.publicKey);
    assert.ok(oracle.mint.equals(mint));
  });

  it("Registers a token", async () => {
    const ix1 = await program.methods.tokenVaultCreate(tokenIndex)
      .accounts({
        market: marketPDA,
        admin: owner.publicKey,
        mint,
        vault: vaultPDA,
        mintInfo: mintInfoPDA,
        payer: owner.publicKey, // This is the payer
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
      1,
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
        admin: owner.publicKey,
        mint,
        bank: bankPDA,
        vault: vaultPDA,
        mintInfo: mintInfoPDA,
        oracle: stubOracle.publicKey,
        fallbackOracle: stubOracle.publicKey,
        payer: owner.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      }).instruction();

    const tx = new anchor.web3.Transaction();
    tx.add(ix1);
    tx.add(ix2);

    const sig = await anchor.web3.sendAndConfirmTransaction(
      provider.connection, tx, [owner]
    );

    console.log("Token registered: ", sig);

    const mintInfoAccount = await program.account.mintInfo.fetch(mintInfoPDA);
    const bankAccount = await program.account.bank.fetch(bankPDA);
    // console.log("Mint Info Account: ", mintInfoAccount);
    // console.log("Bank Account: ", bankAccount);
    assert.ok(mintInfoAccount.mint.equals(mint));
    assert.ok(mintInfoAccount.oracle.equals(stubOracle.publicKey));
  });

  it("Deposits a token", async () => {
    const tx = await program.methods
      .tokenDeposit(deposit_amount, false)
      .accounts({
        market: marketPDA,
        account: peachAccountPDA,
        owner: provider.wallet.publicKey,
        bank: bankPDA,
        vault: vaultPDA,
        oracle: stubOracle.publicKey,
        tokenAccount: tokenAccount,
        tokenAuthority: owner.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([owner, envProviderPayer])
      .rpc();

    const vaultBalance = await program.provider.connection.getTokenAccountBalance(vaultPDA);
    assert.equal(vaultBalance.value.amount, deposit_amount);

    console.log("Token deposited: ", tx);
  });

  it("Withdraws/Borrows a token", async () => {
    const tx = await program.methods
      .tokenWithdraw(withdraw_amount, true)
      .accounts({
        market: marketPDA,
        account: peachAccountPDA,
        owner: provider.wallet.publicKey,
        bank: bankPDA,
        vault: vaultPDA,
        oracle: stubOracle.publicKey,
        tokenAccount: tokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([envProviderPayer])
      .rpc();

    const vaultBalance = await program.provider.connection.getTokenAccountBalance(vaultPDA);
    assert.equal(vaultBalance.value.amount, deposit_amount.sub(withdraw_amount));

    console.log("Token withdrawn: ", tx);
  });

  it("Charges collateral fees", async () => {
    const tx = await program.methods.tokenChargeCollateralFees()
      .accounts({
        market: marketPDA,
        account: peachAccountPDA,
      })
      .rpc();

    console.log("Collateral fee charged: ", tx);
  });

});

