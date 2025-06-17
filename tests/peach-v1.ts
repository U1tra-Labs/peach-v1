import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";
import { Keypair, PublicKey, SystemProgram, Transaction, TransactionInstruction } from "@solana/web3.js";
import { deriveMarketPDA, createTokenAccount, transferToken, provider } from "./helpers/setup";
import { defundWallet, getFundedWallet } from "./helpers/fundWallets";
import { delayForMainnet } from "./helpers/rpc";
import { marketClose, marketCreate } from "./instructions/market";
import { closePeachAccount, createPeachAccount } from "./instructions/peach_account";
import { tokenDeregister, tokenRegister } from "./instructions/token";
import { tokenDeposit, tokenWithdraw, tokenChargeCollateralFees, tokenDepositIntoExisting } from "./instructions/native_lending";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";
import { KAMINO_LENDING_MAIN_MARKET, PYTH_USDC_ORACLE, KAMINO_RESERVE_STATE_USDC, KAMINO_RESERVE_STATE_PYUSD, KAMINO_RESERVE_FARM_STATE_USDC_COLLATERAL, KAMINO_RESERVE_FARM_STATE_USDC_DEBT, KAMINO_RESERVE_FARM_STATE_PYUSD_COLLATERAL, KAMINO_RESERVE_FARM_STATE_PYUSD_DEBT } from "./helpers/const";
import { Program } from "@coral-xyz/anchor";
import kamino_idl from "./idl/kamino_lending.json";
import type { KaminoLending } from "./idl/kamino_lending";
import { Token } from "./objects/token";
import { User } from "./objects/user";
import { getPreInstructions } from "./instructions/kamino/pre_ix";
import { kaminoInitObligation, kaminoInitObligationFarms, kaminoInitUserMetadata } from "./instructions/kamino/init";
import { kaminoBorrow, kaminoDeposit, kaminoRepay, kaminoWithdraw } from "./instructions/kamino/kamino_lending";
import { defaultTokenRegisterParamsPYUSD, defaultTokenRegisterParamsUSDC } from "./objects/token_register_params";
import { TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID } from "@solana/spl-token";
// import { createStubOracle, closeStubOracle } from "./instructions/oracle";

// Test suite
describe("peach-v1", () => {
  const program = anchor.workspace.PeachV1;
  // Will be used to:
  // 1. fund other wallets(admin, any user)
  // 2. create new test tokens
  const programWallet = (program.provider.wallet as NodeWallet).payer; 
  const connection: anchor.web3.Connection = program.provider.connection;

  const kaminoProgram = new Program<KaminoLending>(kamino_idl, provider);
  const kaminoProgramId = kaminoProgram.programId;

  // This test suite is for testing unit instructions from the peach program
  describe("Native happy tests", () => {
    // Owner to the Market
    let admin: Keypair;
    
    // Can be used for testing in mainnet and devnet
    const oracle = PYTH_USDC_ORACLE;

    let market: PublicKey;
    let usdc: Token, pyusd: Token;
    let user1: User, user2: User;
    let usdcATA: PublicKey, pyusdATA: PublicKey; // program wallet's token accounts
    let user1ATA: PublicKey, user2ATA: PublicKey; // usdc
    let user1PyUSDATA: PublicKey, user2PyUSDATA: PublicKey; // pyusd

    // NOTE: Update these values as needed; make sure you change the test cases accordingly
    let marketNum = 0;
    
    // These values are used to test the native deposit and withdraw functions
    let deposit_amount1 = new anchor.BN(0.01 * 10 ** 6), withdraw_amount1 = new anchor.BN(0.005 * 10 ** 6);
    let deposit_amount2 = new anchor.BN(0.01 * 10 ** 6), withdraw_amount2 = new anchor.BN(0.002 * 10 ** 6);
    let borrow_amount = new anchor.BN(0.002 * 10 ** 6), repay_amount = new anchor.BN(0.002 * 10 ** 6);

    before(async () => {
      console.log("Starting Test Setup");

      console.log("Balance before testing: ", await connection.getBalance(programWallet.publicKey));

      admin = await getFundedWallet(0);
      delayForMainnet();

      // Derive PDAs
      market = deriveMarketPDA(marketNum, admin.publicKey);

      usdc = await Token.builder()
        .name("USDC")
        .market(market)
        .tokenIndex(1)
        .programId(TOKEN_PROGRAM_ID)
        .build();
      delayForMainnet();
      
      pyusd = await Token.builder()
        .name("PYUSD")
        .market(market)   
        .tokenIndex(2)
        .programId(TOKEN_2022_PROGRAM_ID)
        .build();
      delayForMainnet();
      // pyusd.display();
      
      user1 = await User.builder()
        .market(market)
        .accountNum(1)
        .build();
      delayForMainnet();
      
      user2 = await User.builder()
        .market(market)
        .accountNum(2)
        .build();
      delayForMainnet();

      // Program wallet's token account
      usdcATA = await createTokenAccount(usdc.mint, programWallet, usdc.programId);
      delayForMainnet();
      pyusdATA = await createTokenAccount(pyusd.mint, programWallet, pyusd.programId);
      delayForMainnet();
      
      // Create or get token accounts for users
      // Transfer some tokens to user on devnet / testnet
      // NOTE: For mainnet make sure program wallet has enough tokens (atleast 0.06 USDC and 0.06 PYUSD)
      user1ATA = await transferToken(usdc.programId, usdc.mint, usdcATA, programWallet, user1.wallet, 0.03 * 10 ** 6);
      delayForMainnet();
      user1PyUSDATA = await transferToken(pyusd.programId, pyusd.mint, pyusdATA, programWallet, user1.wallet, 0.03 * 10 ** 6);
      delayForMainnet();
      user2ATA = await transferToken(usdc.programId, usdc.mint, usdcATA, programWallet, user2.wallet, 0.03 * 10 ** 6);
      delayForMainnet();
      user2PyUSDATA = await transferToken(pyusd.programId, pyusd.mint, pyusdATA, programWallet, user2.wallet, 0.03 * 10 ** 6);

      delayForMainnet();

      user1.addTokenAccounts([user1ATA, user1PyUSDATA])
      user2.addTokenAccounts([user2ATA, user2PyUSDATA])

      console.log(
        "Test setup complete",
        "\nProgram Wallet: ", programWallet.publicKey.toBase58(),
        "\nProgram Wallet USDC ATA: ", usdcATA.toBase58(),
        "\nProgram Wallet PYUSD ATA: ", pyusdATA.toBase58(),
        "\nAdmin: ", admin.publicKey.toBase58(),
        "\nMarket: ", market.toBase58(),
        "\n"
      );

      user1.display();
      user2.display();
      usdc.display();
      pyusd.display();
    
    });

    describe.skip("Create accounts and register token", () => {
      it("Creates a market", async () => {
        try {
          let marketAccount = await program.account.market.fetch(market).catch(() => null);
          if (!marketAccount) {
            const signature = await marketCreate(market, marketNum, admin);
            console.log("Market created with signature: ", signature);
            marketAccount = await program.account.market.fetch(market);
          }

          assert.equal(marketAccount.marketNum, marketNum);
          assert(marketAccount.admin.equals(admin.publicKey));
        }
        catch (err) {
          assert.fail("Errow while creating market: " + err);
        }
        delayForMainnet();
      });

      it("Creates a peach account", async () => {
        try {
          let account1 = await program.account.peachAccountFixed.fetch(user1.peachAccount).catch(() => null);
          let account2 = await program.account.peachAccountFixed.fetch(user2.peachAccount).catch(() => null);

          if (!account1) {
            const signature = await createPeachAccount(user1, market);
            console.log("Peach account created for user1: ", signature);
            account1 = await program.account.peachAccountFixed.fetch(user1.peachAccount);
          }

          if (!account2) {
            const signature = await createPeachAccount(user2, market);
            console.log("Peach account created for user2: ", signature);
            account2 = await program.account.peachAccountFixed.fetch(user2.peachAccount);
          }
          
          assert.equal(account1.accountNum, user1.accountNum);
          assert.equal(account2.accountNum, user2.accountNum);
          assert(account1.owner.equals(user1.wallet.publicKey));
          assert(account2.owner.equals(user2.wallet.publicKey));
        }
        catch (err) {
          assert.fail("Error while creating peach account: " + err);
        }
        delayForMainnet();
      });

      // it("Creates a stub oracle", async () => {
      //   try {
      //     await createStubOracle(stubOracle, market, usdc.mint, oracle_price, admin);
          
      //     const oracle = await program.account.stubOracle.fetch(stubOracle.publicKey);
      //     assert.equal(oracle.price[0].toNumber() / Math.pow(2, 48), oracle_price.toNumber());
      //     assert.ok(oracle.mint.equals(usdc.mint));
      //   }
      //   catch (err) {
      //     assert.fail("Error while creating stub oracle: " + err);
      //   }
      // });

      it("Registers usdc", async () => {
        try {
          let mintInfoAccount = await program.account.mintInfo.fetch(usdc.mint_info).catch(() => null);
          if (!mintInfoAccount) {
            const signature = await tokenRegister(market, usdc, oracle, admin, defaultTokenRegisterParamsUSDC);
            console.log("Registered usdc: ", signature);
            mintInfoAccount = await program.account.mintInfo.fetch(usdc.mint_info);
          }
          
          const bankAccount = await program.account.bank.fetch(usdc.bank[0]);
          assert.ok(mintInfoAccount.mint.equals(usdc.mint));
          assert.ok(mintInfoAccount.oracle.equals(oracle));
          assert.ok(bankAccount.mint.equals(usdc.mint));
          assert.ok(bankAccount.tokenIndex[0] == usdc.tokenIndex);
        }
        catch (err) {
          assert.fail("Error while registering token: " + err);
        }
        delayForMainnet();
      });

      it("Registers pyusd", async () => {
        try {
          let mintInfoAccount = await program.account.mintInfo.fetch(pyusd.mint_info).catch(() => null);
          if (!mintInfoAccount) {
            const signature = await tokenRegister(market, pyusd, oracle, admin, defaultTokenRegisterParamsPYUSD);
            console.log("Registered pyusd: ", signature);
            mintInfoAccount = await program.account.mintInfo.fetch(pyusd.mint_info);
          }          
          const bankAccount = await program.account.bank.fetch(pyusd.bank[0]);
          assert.ok(mintInfoAccount.mint.equals(pyusd.mint));
          assert.ok(mintInfoAccount.oracle.equals(oracle));
          assert.ok(bankAccount.mint.equals(pyusd.mint));
          assert.ok(bankAccount.tokenIndex[0] == pyusd.tokenIndex);
        }
        catch (err) {
          assert.fail("Error while registering token: " + err);
        }
        delayForMainnet();
      });
    });

    describe.skip("Native Token operations", () => {

      it("Deposits usdc: user1", async () => {
        try {
          const vaultBalanceA = new anchor.BN((await connection.getTokenAccountBalance(usdc.vault[0])).value.amount);
          const signature = await tokenDeposit(deposit_amount1, market, user1, usdc, oracle, [usdc]);
          console.log("Deposited usdc for user1: ", signature);
          
          const vaultBalanceB = new anchor.BN((await connection.getTokenAccountBalance(usdc.vault[0])).value.amount);
          
          assert(vaultBalanceB.sub(vaultBalanceA).eq(deposit_amount1));
        } catch (err)  {
          assert.fail("Error while depositing a token: " + err);
        }
        delayForMainnet();
      });
      
      it("Withdraws usdc: user1", async () => {
        try {
          const vaultBalanceA = new anchor.BN((await connection.getTokenAccountBalance(usdc.vault[0])).value.amount);
          const signature = await tokenWithdraw(withdraw_amount1, market, user1, usdc, oracle, [usdc]);
          console.log("Withdrew usdc for user1: ", signature);

          const vaultBalanceB = new anchor.BN((await connection.getTokenAccountBalance(usdc.vault[0])).value.amount);

          assert(vaultBalanceA.sub(withdraw_amount1).eq(vaultBalanceB));
        } catch (err) {
          assert.fail("Error while withdrawing a token: " + err);
        }
      });

      it("Deposits pyusd: user2", async () => {
        try {
          const vaultBalanceA = new anchor.BN((await connection.getTokenAccountBalance(pyusd.vault[0])).value.amount);
          const signature = await tokenDepositIntoExisting(deposit_amount2, market, user2, pyusd, oracle, [pyusd]);
          console.log("Deposited pyusd for user2: ", signature);
          
          const vaultBalanceB = new anchor.BN((await connection.getTokenAccountBalance(pyusd.vault[0])).value.amount);
          
          assert(vaultBalanceB.sub(vaultBalanceA).eq(deposit_amount2));
        } catch (err) {
          assert.fail("Error while depositing a token: " + err);
        }
      });

      it("Withdraws pyusd: user2", async () => {
        try {
          const vaultBalanceA = new anchor.BN((await connection.getTokenAccountBalance(pyusd.vault[0])).value.amount);
          const signature = await tokenWithdraw(withdraw_amount2, market, user2, pyusd, oracle, [pyusd]);
          console.log("Withdrew pyusd for user2: ", signature);

          const vaultBalanceB = new anchor.BN((await connection.getTokenAccountBalance(pyusd.vault[0])).value.amount);

          assert(vaultBalanceA.sub(withdraw_amount2).eq(vaultBalanceB));
        } catch (err) {
          assert.fail("Error while withdrawing a token: " + err);
        }
      });

      it("Borrows usdc: user2", async () => {
        try {
          const vaultBalanceA = new anchor.BN((await connection.getTokenAccountBalance(usdc.vault[0])).value.amount);
          const signature = await tokenWithdraw(borrow_amount, market, user2, usdc, oracle, [usdc]);
          console.log("Borrowed usdc for user2: ", signature);

          const vaultBalanceB = new anchor.BN((await connection.getTokenAccountBalance(usdc.vault[0])).value.amount);

          assert(vaultBalanceA.sub(borrow_amount).eq(vaultBalanceB));
        } catch (err) {
          assert.fail("Error while borrowing a token: " + err);
        }
      });

      it("Repays usdc: user2", async () => {
        try {
          const vaultBalanceA = new anchor.BN((await connection.getTokenAccountBalance(usdc.vault[0])).value.amount);
          const signature = await tokenDeposit(repay_amount, market, user2, usdc, oracle, [usdc, pyusd]);
          console.log("Repayed usdc for user2: ", signature);
          
          const vaultBalanceB = new anchor.BN((await connection.getTokenAccountBalance(usdc.vault[0])).value.amount);
          
          assert(vaultBalanceB.sub(vaultBalanceA).eq(repay_amount));
        } catch (err)  {
          assert.fail("Error while repaying a token: " + err);
        }
        delayForMainnet();
      });

      it("Borrows pyusd: user1", async () => {
        try {
          const vaultBalanceA = new anchor.BN((await connection.getTokenAccountBalance(pyusd.vault[0])).value.amount);
          const signature = await tokenWithdraw(borrow_amount, market, user1, pyusd, oracle, [pyusd]);
          console.log("Borrowed usdc for user1: ", signature);

          const vaultBalanceB = new anchor.BN((await connection.getTokenAccountBalance(pyusd.vault[0])).value.amount);

          assert(vaultBalanceA.sub(borrow_amount).eq(vaultBalanceB));
        } catch (err) {
          assert.fail("Error while borrowing a token: " + err);
        }
      });

      it("Repays pyusd: user1", async () => {
        try {
          const vaultBalanceA = new anchor.BN((await connection.getTokenAccountBalance(pyusd.vault[0])).value.amount);
          const signature = await tokenDeposit(repay_amount, market, user1, pyusd, oracle, [pyusd, usdc]);
          console.log("Repayed usdc for user1: ", signature);
          
          const vaultBalanceB = new anchor.BN((await connection.getTokenAccountBalance(pyusd.vault[0])).value.amount);
          
          assert(vaultBalanceB.sub(vaultBalanceA).eq(repay_amount));
        } catch (err)  {
          assert.fail("Error while repaying a token: " + err);
        }
        delayForMainnet();
      });

      it("Charges collateral fees", async () => {
        try {
          const signature = await tokenChargeCollateralFees(market, user2.peachAccount);
          console.log("Collateral fees charged for user2: ", signature);
        }
        catch (err) {  
          assert.fail("Error while charging collateral fees: " + err);
        }
      });

    });

    // This test suite is for testing Kamino instructions
    // NOTE: To be tested only on mainnet (for cpis)
    describe.skip("Kamino operations", () => {
      
      // Kamino instruction's specific variables
      let lendingMarket: PublicKey;

      const deposit_amount = new anchor.BN(0.01 * 10 ** 6);
      const withdraw_amount = new anchor.BN(0.005 * 10 ** 6);
      const borrow_amount = new anchor.BN(0.002 * 10 ** 6); 
      const repay_amount = new anchor.BN(0.001 * 10 ** 6);
      
      const args = { tag: 0, id: 0 };

      before(async () => {
        lendingMarket = KAMINO_LENDING_MAIN_MARKET;
        usdc.extendToKamino(KAMINO_RESERVE_STATE_USDC, KAMINO_RESERVE_FARM_STATE_USDC_COLLATERAL, KAMINO_RESERVE_FARM_STATE_USDC_DEBT); // Only Collateral Farm State will be used in this test
        pyusd.extendToKamino(KAMINO_RESERVE_STATE_PYUSD, KAMINO_RESERVE_FARM_STATE_PYUSD_COLLATERAL, KAMINO_RESERVE_FARM_STATE_PYUSD_DEBT); // Only Debt Farm State will be used in this test

        await user1.extendToKaminoUser(args, [usdc, pyusd]);

        console.log(
          "Kamino program id: ", kaminoProgramId.toBase58(),
        );
        user1.display();
        usdc.display();
        pyusd.display();
      });

      // NOTE: Skip this test once passed. As Kamino will return error if the user metadata already exists
      it.skip("Init User MetaData", async () => {
        try {
          await kaminoInitUserMetadata(user1, market);

          const data = await kaminoProgram.account.userMetadata.fetch(user1.userMetadata);
          assert.equal(user1.wallet.publicKey.toBase58(), data.owner.toBase58());
          delayForMainnet();
        }
        catch(err) {
          assert.fail("Errow while initializing kamino user metadata: " + err);
        }
      });

      // NOTE: Skip this test once passed. As Kamino will return error if the obligation already exists
      it.skip("Init Obligation", async () => {
        try {
          await kaminoInitObligation(user1, market, args);

          const obligationAccount = await kaminoProgram.account.obligation.fetch(user1.obligation);
          assert.equal(obligationAccount.owner.toBase58(), user1.wallet.publicKey.toBase58());
          assert.equal(obligationAccount.lendingMarket.toBase58(), lendingMarket.toBase58());
          assert.equal(obligationAccount.tag, args.tag);
          delayForMainnet();
        }
        catch(err) {
          assert.fail("Errow while initializing kamino obligation: " + err);
        }
      });

      // Note: Skip this test once passed. As Kamino will return error if obligation farms already exists
      it.skip("Init User Obligation Farm for Reserve", async () => {
        try {
          // Here we use usdc as collateral and pyusd as debt. 
          // Incase of any deposit initialize the collateral farm using mode 0
          // and for borrow initialize debt farm using mode 1.
          await kaminoInitObligationFarms(user1, usdc, market, 0); // 0 for Collateral Farm
          await kaminoInitObligationFarms(user1, pyusd, market, 1); // 1 for Debt Farm

          delayForMainnet();
        }
        catch(err) {
          assert.fail("Errow while initializing user obligation farm for reserve: " + err);
        }
      });

      it("Deposit to kamino", async () => {
        try {
          // usdc reserve refreshed
          // No obligation farms are refreshed.
          const preIx: TransactionInstruction[] = await getPreInstructions(user1, usdc, [usdc.reserve], []);
          
          await kaminoDeposit(
            deposit_amount,
            user1,
            usdc,
            market,
            oracle,
            preIx, // Note: pass [] in devnet
            [pyusd, usdc]
          );

          delayForMainnet();
        }
        catch (err) {
          assert.fail("Error while depositing to kamino: " + err);
        }         
      });

      it("Withdraw from kamino", async () => {
        try {
          // usdc reserve are refereshed as it is used in withdraw
          // usdc obligation farms are refreshed as an obligation was created in deposit
          const preIx: TransactionInstruction[] = await getPreInstructions(user1, usdc, [usdc.reserve], [usdc.reserve]);

          await kaminoWithdraw(
            withdraw_amount,
            user1,
            usdc,
            market,
            oracle,
            preIx, // Note: pass [] in devnet
            [pyusd, usdc]
          );
          
          delayForMainnet();
        }
        catch (err) {
          assert.fail("Error while withdrawing from kamino: " + err);
        }
      });

      it("Borrow from kamino", async () => {
        try {
          // usdc reserve and pyusd reserve are refereshed as pyusd is used in borrow
          // usdc obligation farms are refreshed as an obligation was created in deposit
          const preIx: TransactionInstruction[] = await getPreInstructions(user1, pyusd, [usdc.reserve, pyusd.reserve], [usdc.reserve]);

          await kaminoBorrow(
            borrow_amount,
            user1,
            pyusd,
            market,
            oracle,
            preIx, // Note: [] for devnet testing
            pyusdATA, // Note: used in devnet
            [pyusd, usdc]
          );

          delayForMainnet();
        }
        catch (err) {
          assert.fail("Error while borrowing from kamino: " + err);
        }
      });

      it("Repay to kamino", async () => {
        try {
          // usdc reserve and pyusd reserve are refereshed
          // usdc obligation farms are refreshed and also pyusd farms as an obligation was created in borrow
          const preIx: TransactionInstruction[] = await getPreInstructions(user1, pyusd, [usdc.reserve, pyusd.reserve], [usdc.reserve, pyusd.reserve]);
  
          await kaminoRepay(
            repay_amount,
            user1,
            pyusd,
            market,
            oracle,
            preIx, // Note: pass [] in devnet
            pyusdATA, // Note: used in devnet
            [pyusd, usdc]
          );
          
          delayForMainnet();
        }
        catch (err) {
          assert.fail("Error while repaying to kamino: " + err);
        }        
      });
    });

    // Do not run this test suite; if you want to preserve the accounts
    describe.skip("Deregister and close accounts", () => {
      // it("Close stub oracle", async () => {
      //   try {
      //     const signature = await closeStubOracle(stubOracle, market, admin);
          
        
      //     assert.ok(await program.account.stubOracle.fetch(oracle.publicKey).then(() => false).catch(() => true));
      //   } catch (err) {
      //     assert.fail("Error while closing stub oracle: " + err);
      //   }
      // });

      it("Close peach account", async () => {
        try {
          const signature1 = await closePeachAccount(user1, market);
          console.log("Peach account closed for user1: ", signature1);
          const signature2 = await closePeachAccount(user2, market);
          console.log("Peach account closed for user2: ", signature2);
        
          assert.ok(await program.account.peachAccountFixed.fetch(user1.peachAccount).then(() => false).catch(() => true));
          assert.ok(await program.account.peachAccountFixed.fetch(user2.peachAccount).then(() => false).catch(() => true));
        } catch (err) {
          assert.fail("Error while closing peach accounts: " + err);
        }
        delayForMainnet();
      });

      it("Deregister usdc", async () => {
        try {
          const signature = await tokenDeregister(market, admin, usdc, usdcATA, programWallet.publicKey);
          console.log("Deregistered usdc: ", signature);
          const mintInfoAccount = await program.account.mintInfo.fetch(usdc.mint_info).then(() => false).catch(() => true);
          const bankAccount = await program.account.bank.fetch(usdc.bank[0]).then(() => false).catch(() => true);
          assert.ok(mintInfoAccount);
          assert.ok(bankAccount);

        } catch (err) {
          assert.fail("Error while deregistering token: " + err);
        }
        delayForMainnet();
      });

      it("Deregister pyusd", async () => {
        try {
          const signature = await tokenDeregister(market, admin, pyusd, pyusdATA, programWallet.publicKey);
          console.log("Deregistered pyusd: ", signature);

          const mintInfoAccount = await program.account.mintInfo.fetch(pyusd.mint_info).then(() => false).catch(() => true);
          const bankAccount = await program.account.bank.fetch(pyusd.bank[0]).then(() => false).catch(() => true);
          assert.ok(mintInfoAccount);
          assert.ok(bankAccount);

        } catch (err) {
          assert.fail("Error while deregistering token: " + err);
        }
        delayForMainnet();
      });

      it("Closes a market", async () => {
        try {
          const signature = await marketClose(market, admin);
          console.log("Market closed: ", signature);
          assert(await program.account.market.fetch(market).then(() => false).catch(() => true));
        } catch (err) {
          assert.fail("Error while closing market: " + err);
        }
        delayForMainnet();
      });
    });

    after(async () => {
      // Note: Comment out; if you want to preserve the accounts
      // Only defunds the SOL
      // console.log("Defunding wallets");
      // await defundWallet(0);
      // await defundWallet(1);
      // await defundWallet(2);
      // console.log("Wallets defunded");

      console.log("Balance after testing: ", await connection.getBalance(programWallet.publicKey));
    });

  });
});
