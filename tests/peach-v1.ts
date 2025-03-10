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
import { TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID } from "@solana/spl-token";

// Test suite
describe("peach-v1", () => {
  const program = anchor.workspace.PeachV1;
  const programWallet = (program.provider.wallet as NodeWallet).payer;
  const stubOracle = Keypair.generate();
  let admin: Keypair;
  let user1: Keypair;
  // let user2: Keypair, user3: Keypair;
  let market1: PublicKey, peachAccount1: PublicKey, bank1: PublicKey, vault1: PublicKey, mintInfo1: PublicKey;
  let mint: PublicKey;
  let programTokenAccount1: PublicKey, user1TokenAccount1: PublicKey;
  let connection: anchor.web3.Connection = program.provider.connection;

  // Update these values as needed
  let marketNum = 1, accountNum = 0, tokenIndex = 1, price = 1.0;
  let deposit_amount1 = new anchor.BN(100), deposit_amount2 = new anchor.BN(50);
  let withdraw_amount = new anchor.BN(5);

  before(async () => {
    console.log("Starting Test Setup");

    admin = await getFundedWallet();
    user1 = await getFundedWallet();
    // user2 = await getFundedWallet();
    // user3 = await getFundedWallet();

    // Create mint from our program wallet
    mint = await createTokenMint(10, programWallet);

    // Create token accounts
    programTokenAccount1 = await createTokenAccount(mint, programWallet);

    // Transfer some tokens to user1
    user1TokenAccount1 = await transferToken(mint, programWallet, user1, 200);

    // Derive PDAs
    market1 = deriveMarketPDA(marketNum, admin.publicKey);
    peachAccount1 = derivePeachAccountPDA(market1, accountNum, user1.publicKey);
    bank1 = deriveBankPDA(market1, tokenIndex);
    vault1 = deriveVaultPDA(market1, tokenIndex);
    mintInfo1 = deriveMintInfoPDA(market1, mint);

    console.log("Test setup complete\n Market: ", market1.toBase58(), "\n User Account: ", peachAccount1.toBase58(), "\n Bank: ", bank1.toBase58(), "\n Vault: ", vault1.toBase58(), "\n Mint Info: ", mintInfo1.toBase58());
    console.log("Admin: ", admin.publicKey.toBase58(), "\n User1: ", user1.publicKey.toBase58());
  });

  describe("Basic Happy Tests", () => {
    it("Creates a market", async () => {
      try {
        await marketCreate(market1, marketNum, admin);
        
        const market = await program.account.market.fetch(market1);
        assert.equal(market.marketNum, marketNum);
        assert(market.admin.equals(admin.publicKey));
      }
      catch (err) {
        assert.fail("Errow while creating market: " + err);
      }
    });

    it("Creates a peach account", async () => {
      try {
        await createPeachAccount(peachAccount1, market1, accountNum, user1);
        
        const account = await program.account.peachAccount.fetch(peachAccount1);
        assert.equal(account.accountNum, accountNum);
        assert(account.owner.equals(user1.publicKey));
      }
      catch (err) {
        assert.fail("Error while creating peach account: " + err);
      }
    });

    it("Creates a stub oracle", async () => {
      try {
        await createStubOracle(stubOracle, market1, mint, price, admin);
        
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
        await tokenRegister(tokenIndex, mint, market1, vault1, mintInfo1, bank1, stubOracle.publicKey, admin);
        
        const mintInfoAccount = await program.account.mintInfo.fetch(mintInfo1);
        const bankAccount = await program.account.bank.fetch(bank1);
        const vault = await connection.getParsedAccountInfo(vault1);
        // console.log("Bank: ", bankAccount, "\n Vault: ", vault.value.data);
        assert.ok(vault.value.data.parsed.info.owner == market1.toBase58());
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
        await tokenDeposit(deposit_amount1, market1, peachAccount1, bank1, vault1, stubOracle.publicKey, user1TokenAccount1, user1);
        
        const vaultBalance = await program.provider.connection.getTokenAccountBalance(vault1);
        const userAccount = await program.account.peachAccount.fetch(peachAccount1);
        assert.equal(userAccount.netDeposits.toNumber(), deposit_amount1);
        assert.equal(vaultBalance.value.amount, deposit_amount1);
      } catch (err)  {
        assert.fail("Error while depositing a token: " + err);
      }
    });

    it("Withdraws/Borrows a token", async () => {
      try {
        await tokenWithdraw(withdraw_amount, market1, peachAccount1, bank1, vault1, stubOracle.publicKey, user1TokenAccount1);

        const vaultBalance = await program.provider.connection.getTokenAccountBalance(vault1);
        const peachAccount = await program.account.peachAccount.fetch(peachAccount1);
        assert.equal(peachAccount.netDeposits.toNumber(), deposit_amount1.sub(withdraw_amount));
        assert.equal(vaultBalance.value.amount, deposit_amount1.sub(withdraw_amount));
      } catch (err) {
        assert.fail("Error while withdrawing a token: " + err);
      }
    });

    it("Deposits a token again", async () => {
      try{
        await tokenDepositIntoExisting(deposit_amount2, market1, peachAccount1, bank1, vault1, stubOracle.publicKey, user1TokenAccount1, user1);
        
        const vaultBalance = await program.provider.connection.getTokenAccountBalance(vault1);
        const userAccount = await program.account.peachAccount.fetch(peachAccount1);
        assert.equal(userAccount.netDeposits.toNumber(), deposit_amount2.add(deposit_amount1).sub(withdraw_amount).toNumber());
        assert.equal(vaultBalance.value.amount, deposit_amount2.add(deposit_amount1).sub(withdraw_amount).toNumber());
      } catch (err)  {
        assert.fail("Error while depositing a token: " + err);
      }
    });

    it("Charges collateral fees", async () => {
      try {
        await tokenChargeCollateralFees(market1, peachAccount1);
      }
      catch (err) {  
        assert.fail("Error while charging collateral fees: " + err);
      }
    });

    it("Deregister a token", async () => {
      try {
        await tokenDeregister(market1, admin, mintInfo1, programTokenAccount1, programWallet.publicKey, bank1, vault1);

        const mintInfo = await program.account.mintInfo.fetch(mintInfo1).then(() => false).catch(() => true);
        const bank = await program.account.bank.fetch(bank1).then(() => false).catch(() => true); 
        assert.ok(mintInfo);
        assert.ok(bank);

      } catch (err) {
        assert.fail("Error while deregistering token: " + err);
      }
    });

    it("Closes a market", async () => {
      try {
        await marketClose(market1, admin);
        assert(await program.account.market.fetch(market1).then(() => false).catch(() => true));
      } catch (err) {
        assert.fail("Error while closing market: " + err);
      }
      
    });
  });

  after(async () => {
    console.log("Defunding wallets");
    await defundWallet(admin);
    await defundWallet(user1);
    // await defundWallet(user2);
    // await defundWallet(user3);
    console.log("Wallets defunded");
  });

});

