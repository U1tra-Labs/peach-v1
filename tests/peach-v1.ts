import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";
import { Keypair, PublicKey } from "@solana/web3.js";
import { createTokenMint, deriveMarketPDA, derivePeachAccountPDA, deriveBankPDA, deriveVaultPDA, deriveMintInfoPDA, createTokenAccount, transferToken } from "./helpers/setup";
import { defundWallet, getFundedWallet } from "./helpers/fundWallets";
import { marketClose, marketCreate } from "./instructions/market";
import { createPeachAccount } from "./instructions/peach_account";
import { createStubOracle } from "./instructions/oracle";
import { tokenChargeCollateralFees, tokenDeposit, tokenDepositIntoExisting, tokenDeregister, tokenForceWithdraw, tokenRegister, tokenWithdraw } from "./instructions/token";
import { I80F48 } from "./helpers/I80F48";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";

// Test suite
describe("peach-v1", () => {
  const program = anchor.workspace.PeachV1;
  const programWallet = (program.provider.wallet as NodeWallet).payer;

  describe("Basic happy tests", () => {
    const stubOracle = Keypair.generate();
    let admin: Keypair;
    let user: Keypair;
    let market: PublicKey, peachAccount: PublicKey, bank: PublicKey, vault: PublicKey, mintInfo: PublicKey;
    let mint: PublicKey;
    let programTokenAccount1: PublicKey, userTokenAccount1: PublicKey;
    let connection: anchor.web3.Connection = program.provider.connection;

    // Update these values as needed
    let marketNum = 0, accountNum = 0, tokenIndex = 0, price = 1.0;
    let deposit_amount1 = new anchor.BN(100), deposit_amount2 = new anchor.BN(50);
    let withdraw_amount = new anchor.BN(5);

    before(async () => {
    console.log("Starting Test Setup");

    admin = await getFundedWallet();
    user = await getFundedWallet();

    // Create mint from our program wallet
    mint = await createTokenMint(10, programWallet);

    // Create token accounts
    programTokenAccount1 = await createTokenAccount(mint, programWallet);

    // Transfer some tokens to user
    userTokenAccount1 = await transferToken(mint, programWallet, user, 200);

    // Derive PDAs
    market = deriveMarketPDA(marketNum, admin.publicKey);
    peachAccount = derivePeachAccountPDA(market, accountNum, user.publicKey);
    bank = deriveBankPDA(market, tokenIndex);
    vault = deriveVaultPDA(market, tokenIndex);
    mintInfo = deriveMintInfoPDA(market, mint);

    console.log("Test setup complete\n Market: ", market.toBase58(), "\n User Account: ", peachAccount.toBase58(), "\n Bank: ", bank.toBase58(), "\n Vault: ", vault.toBase58(), "\n Mint Info: ", mintInfo.toBase58());
    console.log("Admin: ", admin.publicKey.toBase58(), "\n User1: ", user.publicKey.toBase58());
    });
    
    it("Creates a market", async () => {
      try {
        await marketCreate(market, marketNum, admin);
        
        const marketAccount = await program.account.market.fetch(market);
        assert.equal(marketAccount.marketNum, marketNum);
        assert(marketAccount.admin.equals(admin.publicKey));
      }
      catch (err) {
        assert.fail("Errow while creating market: " + err);
      }
    });

    it("Creates a peach account", async () => {
      try {
        await createPeachAccount(peachAccount, market, accountNum, user);
        
        const account = await program.account.peachAccount.fetch(peachAccount);
        assert.equal(account.accountNum, accountNum);
        assert(account.owner.equals(user.publicKey));
      }
      catch (err) {
        assert.fail("Error while creating peach account: " + err);
      }
    });

    it("Creates a stub oracle", async () => {
      try {
        await createStubOracle(stubOracle, market, mint, price, admin);
        
        const oracle = await program.account.stubOracle.fetch(stubOracle.publicKey);
        assert.equal(oracle.price.val.toNumber(), I80F48.fromNumber(price).getData().toNumber());
        assert.ok(oracle.mint.equals(mint));
      }
      catch (err) {
        assert.fail("Error while creating stub oracle: " + err);
      }
    });

    it("Registers a token", async () => {
      try {
        await tokenRegister(tokenIndex, mint, market, vault, mintInfo, bank, stubOracle.publicKey, admin);
        
        const mintInfoAccount = await program.account.mintInfo.fetch(mintInfo);
        const bankAccount = await program.account.bank.fetch(bank);
        const vaultAccount = await connection.getParsedAccountInfo(vault);
        // console.log("Bank: ", bankAccount, "\n Vault: ", vault.value.data);
        assert.ok(vaultAccount.value.data.parsed.info.owner == market.toBase58());
        assert.ok(mintInfoAccount.mint.equals(mint));
        assert.ok(mintInfoAccount.oracle.equals(stubOracle.publicKey));
        assert.ok(bankAccount.mint.equals(mint));
        assert.ok(bankAccount.tokenIndex == tokenIndex);
      }
      catch (err) {
        assert.fail("Error while registering token: " + err);
      }
    });

    it("Deposits a token", async () => {
      try{
        await tokenDeposit(deposit_amount1, market, peachAccount, bank, vault, stubOracle.publicKey, userTokenAccount1, user);
        
        const vaultBalance = await program.provider.connection.getTokenAccountBalance(vault);
        const userAccount = await program.account.peachAccount.fetch(peachAccount);
        assert.equal(userAccount.netDeposits.toNumber(), deposit_amount1);
        assert.equal(vaultBalance.value.amount, deposit_amount1);
      } catch (err)  {
        assert.fail("Error while depositing a token: " + err);
      }
    });

    it("Withdraws/Borrows a token", async () => {
      try {
        await tokenWithdraw(withdraw_amount, market, peachAccount, bank, vault, stubOracle.publicKey, userTokenAccount1);

        const vaultBalance = await program.provider.connection.getTokenAccountBalance(vault);
        const peachAccount1 = await program.account.peachAccount.fetch(peachAccount);
        assert.equal(peachAccount1.netDeposits.toNumber(), deposit_amount1.sub(withdraw_amount));
        assert.equal(vaultBalance.value.amount, deposit_amount1.sub(withdraw_amount));
      } catch (err) {
        assert.fail("Error while withdrawing a token: " + err);
      }
    });

    it("Deposits a token again", async () => {
      try{
        await tokenDepositIntoExisting(deposit_amount2, market, peachAccount, bank, vault, stubOracle.publicKey, userTokenAccount1, user);
        
        const vaultBalance = await program.provider.connection.getTokenAccountBalance(vault);
        const userAccount = await program.account.peachAccount.fetch(peachAccount);
        assert.equal(userAccount.netDeposits.toNumber(), deposit_amount2.add(deposit_amount1).sub(withdraw_amount).toNumber());
        assert.equal(vaultBalance.value.amount, deposit_amount2.add(deposit_amount1).sub(withdraw_amount).toNumber());
      } catch (err)  {
        assert.fail("Error while depositing a token: " + err);
      }
    });

    it("Charges collateral fees", async () => {
      try {
        await tokenChargeCollateralFees(market, peachAccount);
      }
      catch (err) {  
        assert.fail("Error while charging collateral fees: " + err);
      }
    });

    it("Close stub oracle", async () => {
      try {
        await program.methods.stubOracleClose()
        .accounts({
            market: market,
            admin: admin.publicKey,
            oracle: stubOracle.publicKey,
            solDestination: programWallet.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
          }
        )
        .signers([admin])
        .rpc();
        
        assert.ok(await program.account.stubOracle.fetch(stubOracle.publicKey).then(() => false).catch(() => true));
      } catch (err) {
        assert.fail("Error while closing stub oracle: " + err);
      }
    });

    it("Close peach account", async () => {
      try {
        await program.methods.accountClose(true)
        .accounts({
            market: market,
            account: peachAccount,
            owner: user.publicKey,
            solDestination: programWallet.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
          }
        )
        .signers([user])
        .rpc();
        
        assert.ok(await program.account.peachAccount.fetch(peachAccount).then(() => false).catch(() => true));
      } catch (err) {
        assert.fail("Error while closing stub oracle: " + err);
      }
    });

    it("Deregister a token", async () => {
      try {
        await tokenDeregister(market, admin, mintInfo, programTokenAccount1, programWallet.publicKey, bank, vault);

        const mintInfoAccount = await program.account.mintInfo.fetch(mintInfo).then(() => false).catch(() => true);
        const bankAccount = await program.account.bank.fetch(bank).then(() => false).catch(() => true); 
        assert.ok(mintInfoAccount);
        assert.ok(bankAccount);

      } catch (err) {
        assert.fail("Error while deregistering token: " + err);
      }
    });

    it("Closes a market", async () => {
      try {
        await marketClose(market, admin);
        assert(await program.account.market.fetch(market).then(() => false).catch(() => true));
      } catch (err) {
        assert.fail("Error while closing market: " + err);
      }
      
    });

    after(async () => {
      console.log("Defunding wallets");
      await defundWallet(admin);
      await defundWallet(user);
      console.log("Wallets defunded");
    });

  });

  describe("Borrow limit tests", () => {
    // TODO
  });
});