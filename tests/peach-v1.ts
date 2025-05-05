import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { createTokenMint, deriveMarketPDA, derivePeachAccountPDA, deriveBankPDA, deriveVaultPDA, deriveMintInfoPDA, createTokenAccount, transferToken, provider, testDerivePeachAccountPDA } from "./helpers/setup";
import { defundWallet, getFundedWallet } from "./helpers/fundWallets";
import { marketClose, marketCreate } from "./instructions/market";
import { createPeachAccount } from "./instructions/peach_account";
import { createStubOracle } from "./instructions/oracle";
import { tokenChargeCollateralFees, tokenDeposit, tokenDepositIntoExisting, tokenDeregister, tokenForceWithdraw, tokenRegister, tokenWithdraw } from "./instructions/token";
import { I80F48 } from "./helpers/I80F48";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";
import { getAssociatedTokenAddress, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { KAMINO_FARM_MAINNET, KAMINO_LENDING_MAIN_MARKET, KAMINO_LENDING, KAMINO_RESERVE_FARM_STATE_USDC, KAMINO_RESERVE_USDC, KAMINO_RESERVED_USDC_MINT, TEST_ENV, USDC_MINT_MAINNET } from "./helpers/const";
import { Idl, Program } from "@coral-xyz/anchor";
import { createLookupTableAddress } from "./helpers/create_alt";
import kamino_idl from "./kamino/kamino_lending.json";
import { ASSOCIATED_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";

// Test suite
describe("peach-v1", () => {
  const program = anchor.workspace.PeachV1;
  // Will be used to:
  // 1. fund other wallets(admin, any user)
  // 2. create new test tokens
  const programWallet = (program.provider.wallet as NodeWallet).payer; 
  const connection: anchor.web3.Connection = program.provider.connection;

  const idl = kamino_idl as Idl;
  const kaminoProgram = new Program(idl, KAMINO_LENDING, provider);

  // This test suite is for testing unit instructions from the peach program
  describe("Native happy tests", () => {
    let admin: Keypair;
    let user: Keypair;
    const stubOracle = Keypair.generate(); // This is a keypair for the stub oracle, used for testing

    let market: PublicKey, peachAccount: PublicKey, bank: PublicKey, vault: PublicKey, mintInfo: PublicKey;
    
    let usdc_mint: PublicKey;
    let usdcATA: PublicKey, user1ATA: PublicKey;
    let userTokenAccount: PublicKey;
    let user_kamino_usdc_token_account: PublicKey;
    let reserveLiquiditySupplyPda: PublicKey;
    let reserveDepositCollateralPda: PublicKey;

    // Update these values as needed; make sure you change the test cases accordingly
    // These values are used to derive the PDAs for the peach account, bank, vault, and mint info
    // These values are used to test the deposit and withdraw functions
    let marketNum = 0, accountNum = 0, tokenIndex = 0, price = 1.0;
    let deposit_amount1 = new anchor.BN(100), deposit_amount2 = new anchor.BN(50);
    let withdraw_amount = new anchor.BN(5);

    before(async () => {
      console.log("Starting Test Setup");

      console.log("Balance before testing: ", await connection.getBalance(programWallet.publicKey));

      admin = await getFundedWallet();
      // Wait for the wallet to be funded
      setTimeout(() => {}, 5000);
      user = await getFundedWallet();

      // Create mint from our program wallet for local/devnet testing
      // For mainnet, use USDC mint
      setTimeout(() => {}, 5000);
      usdc_mint = USDC_MINT_MAINNET;
      
      // Create token accounts
      setTimeout(() => { }, 5000);
      // usdcATA = await createTokenAccount(usdc_mint, programWallet);
      usdcATA = new PublicKey("Dc22vGVRbk5dVucRLrwA5UJBuZFKufF4W4kf5pvjLDMZ"); // program wallet USDC token account

      // Transfer some tokens to user
      setTimeout(() => {}, 5000);
      user1ATA = await transferToken(usdc_mint, usdcATA, programWallet, user, 0.01 * 10 ** 6);

      // Derive PDAs
      market = deriveMarketPDA(marketNum, admin.publicKey);
      peachAccount = derivePeachAccountPDA(market, accountNum, user.publicKey);
      bank = deriveBankPDA(market, tokenIndex);
      vault = deriveVaultPDA(market, tokenIndex);
      mintInfo = deriveMintInfoPDA(market, usdc_mint);

      console.log(
        "Test setup complete\n Market: ",
        market.toBase58(), "\n User Account: ",
        peachAccount.toBase58(), "\n Bank: ",
        bank.toBase58(), "\n Vault: ",
        vault.toBase58(), "\n Mint Info: ",
        mintInfo.toBase58(), "\n USDC Mint: ",
        usdc_mint.toBase58()
      );
      console.log(
        "Admin: ", admin.publicKey.toBase58(),
        "\n User1: ", user.publicKey.toBase58()
      );
    
    });

    describe("Create accounts and register token", () => {
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
          await createStubOracle(stubOracle, market, usdc_mint, price, admin);
          
          const oracle = await program.account.stubOracle.fetch(stubOracle.publicKey);
          assert.equal(oracle.price.val.toNumber(), I80F48.fromNumber(price).getData().toNumber());
          assert.ok(oracle.mint.equals(usdc_mint));
        }
        catch (err) {
          assert.fail("Error while creating stub oracle: " + err);
        }
      });

      it("Registers a token", async () => {
        try {
          await tokenRegister(tokenIndex, usdc_mint, market, vault, mintInfo, bank, stubOracle.publicKey, admin);
          
          const mintInfoAccount = await program.account.mintInfo.fetch(mintInfo);
          const bankAccount = await program.account.bank.fetch(bank);
          const vaultAccount = await connection.getParsedAccountInfo(vault);
          // console.log("Bank: ", bankAccount, "\n Vault: ", vault.value.data);
          assert.ok(vaultAccount.value.data.parsed.info.owner == market.toBase58());
          assert.ok(mintInfoAccount.mint.equals(usdc_mint));
          assert.ok(mintInfoAccount.oracle.equals(stubOracle.publicKey));
          assert.ok(bankAccount.mint.equals(usdc_mint));
          assert.ok(bankAccount.tokenIndex == tokenIndex);
        }
        catch (err) {
          assert.fail("Error while registering token: " + err);
        }
      });
    });

    describe.skip("Native Token operations", () => {

      it("Deposits a token", async () => {
        try{
          await tokenDeposit(deposit_amount1, market, peachAccount, bank, vault, stubOracle.publicKey, user1ATA, user);
          
          const vaultBalance = await connection.getTokenAccountBalance(vault);
          const userAccount = await program.account.peachAccount.fetch(peachAccount);
          assert.equal(userAccount.netDeposits.toNumber(), deposit_amount1);
          assert.equal(vaultBalance.value.amount, deposit_amount1);
        } catch (err)  {
          assert.fail("Error while depositing a token: " + err);
        }
      });

      it("Withdraws/Borrows a token", async () => {
        try {
          await tokenWithdraw(withdraw_amount, market, peachAccount, bank, vault, stubOracle.publicKey, user1ATA);

          const vaultBalance = await connection.getTokenAccountBalance(vault);
          const peachAccount1 = await program.account.peachAccount.fetch(peachAccount);
          assert.equal(peachAccount1.netDeposits.toNumber(), deposit_amount1.sub(withdraw_amount));
          assert.equal(vaultBalance.value.amount, deposit_amount1.sub(withdraw_amount));
        } catch (err) {
          assert.fail("Error while withdrawing a token: " + err);
        }
      });

      it("Deposits a token again", async () => {
        try{
          await tokenDepositIntoExisting(deposit_amount2, market, peachAccount, bank, vault, stubOracle.publicKey, user1ATA, user);
          
          const vaultBalance = await connection.getTokenAccountBalance(vault);
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

    });

    // This test suite is for testing Kamino instructions, to be done only on mainnet
    describe("Kamino operations", () => {
      
      // Kamino instruction's specific variables
      let userMetaDataPDA: PublicKey;
      let referrerUserMetaDataPDA: PublicKey;
      let lendingMarketAuthorityPDA: PublicKey;
      let obligationFarm: PublicKey;
      let obligationPDA: PublicKey;
      let lendingMarket: PublicKey;
      let seed1Account: PublicKey, seed2Account: PublicKey;
      

      const deposit_amount = new anchor.BN(0.01 * 10 ** 6);
      const mode = 0;

      // Instruction: InitObligation
      interface InitObligationArgs {
          tag: number; // u8 in Rust -> number in TS (0-255)
          id: number;  // u8 in Rust -> number in TS (0-255)
      }

      const args: InitObligationArgs = {
          tag: 0, 
          id: 0,
      };

      before(async () => {
        referrerUserMetaDataPDA = KAMINO_LENDING; // Switch this to the actual referrer user metadata PDA
        lendingMarket = KAMINO_LENDING_MAIN_MARKET;
        seed1Account = SystemProgram.programId;
        seed2Account = SystemProgram.programId;
        
        [userMetaDataPDA] = PublicKey.findProgramAddressSync(
          [Buffer.from("user_meta"), peachAccount.toBuffer()],
          KAMINO_LENDING
        );

        [obligationPDA] = PublicKey.findProgramAddressSync(
          [
              Buffer.from(Uint8Array.of(args.tag)),
              Buffer.from(Uint8Array.of(args.id)),
              peachAccount.toBuffer(),
              lendingMarket.toBuffer(),
              seed1Account.toBuffer(),
              seed2Account.toBuffer(),
          ],
          KAMINO_LENDING
        );

        [userTokenAccount] = PublicKey.findProgramAddressSync(
          [
              Buffer.from("utc"),
              peachAccount.toBuffer(),
              USDC_MINT_MAINNET.toBuffer(),
          ],
          program.programId
        );
      
        user_kamino_usdc_token_account = await getAssociatedTokenAddress(
            KAMINO_RESERVED_USDC_MINT,
            peachAccount,
            true,
        );

        [reserveLiquiditySupplyPda] = PublicKey.findProgramAddressSync(
            [
                Buffer.from("reserve_liq_supply"),
                KAMINO_LENDING_MAIN_MARKET.toBuffer(),
                USDC_MINT_MAINNET.toBuffer(),
            ],
            kaminoProgram.programId
        );

        [reserveDepositCollateralPda] = PublicKey.findProgramAddressSync(
            [
                Buffer.from("reserve_coll_supply"),
                KAMINO_LENDING_MAIN_MARKET.toBuffer(),
                USDC_MINT_MAINNET.toBuffer(),
            ],
            kaminoProgram.programId
        );

        [lendingMarketAuthorityPDA] = PublicKey.findProgramAddressSync(
          [
            Buffer.from("lma"), 
            lendingMarket.toBuffer(),
          ],
          KAMINO_LENDING
        );

        [obligationFarm] = PublicKey.findProgramAddressSync(
            [
                Buffer.from("user"), 
                KAMINO_RESERVE_FARM_STATE_USDC.toBuffer(),
                obligationPDA.toBuffer(),
            ],
            KAMINO_FARM_MAINNET
        )

        console.log("Kamino program id: ", kaminoProgram.programId.toBase58());
        console.log("User MetaData PDA:", userMetaDataPDA.toBase58());
        console.log("Obligation PDA address: ", obligationPDA.toBase58());
        console.log("User Token Account", userTokenAccount.toBase58());
        console.log("User Kamino USDC Token Account: ", user_kamino_usdc_token_account.toBase58());
        console.log("Reserve Liquidity Supply PDA: ", reserveLiquiditySupplyPda.toBase58());
        console.log("Reserve Deposit Collateral PDA: ", reserveDepositCollateralPda.toBase58());
        console.log("Lending Market Authority PDA: ", lendingMarketAuthorityPDA.toBase58());
        console.log("Obligation Farm PDA: ", obligationFarm.toBase58());
      });

      it("Init User MetaData", async () => {
        const lookupTableAddress = await createLookupTableAddress(program.provider.connection, provider.wallet as NodeWallet);
        
        const signature = await program.methods.kaminoInitUserMetadata(lookupTableAddress)
          .accounts({
              payer: user.publicKey,
              market: market, // market
              peachAccount: peachAccount, // peach account
              owner: user.publicKey,
              userMetadata: userMetaDataPDA,
              referrerUserMetadata: referrerUserMetaDataPDA,
              systemProgram: SystemProgram.programId,
              rent: anchor.web3.SYSVAR_RENT_PUBKEY
          })
        .signers([user])
        .rpc();

        console.log("Init User Metadata signature: ", signature);
        const data = await kaminoProgram.account.userMetadata.fetch(userMetaDataPDA);
        assert.equal(lookupTableAddress.toBase58(), data.userLookupTable.toBase58());
        assert.equal(peachAccount.toBase58(), data.owner.toBase58());

      });

      it("Init Obligation", async () => {

        const signature = await program.methods.kaminoInitObligation(args)
            .accounts({
                payer: user.publicKey,
                market: market,
                peachAccount: peachAccount,
                obligation: obligationPDA,
                lendingMarket: lendingMarket,
                seedOneAccount: seed1Account,
                seedTwoAccount: seed2Account,
                ownerUserMetadata: userMetaDataPDA,
                rent: anchor.web3.SYSVAR_RENT_PUBKEY,
                systemProgram: SystemProgram.programId,
                kaminoProgram: KAMINO_LENDING,
            })
          .signers([user])
          .rpc();

        console.log("Init Obligation transaction signature: ", signature);
        const obligationAccount = await kaminoProgram.account.obligation.fetch(obligationPDA);
        assert.equal(obligationAccount.owner.toBase58(), peachAccount.toBase58());
        assert.equal(obligationAccount.lendingMarket.toBase58(), lendingMarket.toBase58());
        assert.equal(obligationAccount.tag, args.tag);
      });

      it("Init User Obligation Farm for Reserve", async () => {
        
        const signature = await program.methods.kaminoInitObligationFarmForReserve(mode)
          .accounts({
            payer: user.publicKey,
            market: market,
            peachAccount: peachAccount,
            obligation: obligationPDA,
            lendingMarketAuthority: lendingMarketAuthorityPDA,
            reserve: KAMINO_RESERVE_USDC, 
            reserveFarmState: KAMINO_RESERVE_FARM_STATE_USDC, // Derive if needed
            obligationFarm: obligationFarm, // Derive if needed
            lendingMarket: lendingMarket,
            farmsProgram: KAMINO_FARM_MAINNET, 
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
            systemProgram: SystemProgram.programId,
            kaminoProgram: KAMINO_LENDING
          })
        .signers([user])
        .rpc();

        console.log("Init Obligation Farm transaction signature", signature);

      });

      it.skip("Deposit to kamino", async () => {
        
        const signature = await program.methods.kaminoDeposit(
            deposit_amount,
            0,
        ).accounts({
            signer: user.publicKey,
            obligation: obligationPDA,
            peachAccount: peachAccount,
            bank: bank,
            oracle: stubOracle.publicKey,
            market: market,
            kaminoCollateralMint: KAMINO_RESERVED_USDC_MINT,
            mint: USDC_MINT_MAINNET,
            userKaminoReserveUsdcTokenAccount: user_kamino_usdc_token_account,
            userTokenAccount: user1ATA,
            kaminoReserve: KAMINO_RESERVE_USDC,
            lendingMarket: KAMINO_LENDING_MAIN_MARKET,
            lendingMarketAuthority: lendingMarketAuthorityPDA,
            kaminoDestinationDepositCollateral: reserveDepositCollateralPda,
            kaminoReserveLiquidityUsdcSupply: reserveLiquiditySupplyPda,
            kaminoProgram: kaminoProgram.programId,
            farmsProgram: KAMINO_FARM_MAINNET,
            kaminoReserveFarmState: KAMINO_RESERVE_FARM_STATE_USDC,
            collateralTokenProgram: TOKEN_PROGRAM_ID,
            liquidityTokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId, 
            associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
            instructionsSysvar: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
        })
        .signers([user])
        .rpc();
              
        console.log("signature: ", signature);
          
      });

      it("Withdraw from kamino", async() => {

        const signature = await program.methods.kaminoWithdraw(
            deposit_amount,
            0,
        ).accounts({
            signer: user.publicKey,
            obligation: obligationPDA,
            peachAccount: peachAccount,
            bank: bank,
            oracle: stubOracle.publicKey,
            market: market,
            kaminoCollateralMint: KAMINO_RESERVED_USDC_MINT,
            mint: USDC_MINT_MAINNET,
            userKaminoReserveUsdcTokenAccount: user_kamino_usdc_token_account,
            userTokenAccount: user1ATA,
            kaminoReserve: KAMINO_RESERVE_USDC,
            lendingMarket: KAMINO_LENDING_MAIN_MARKET,
            lendingMarketAuthority: lendingMarketAuthorityPDA,
            kaminoDestinationDepositCollateral: reserveDepositCollateralPda,
            kaminoReserveLiquidityUsdcSupply: reserveLiquiditySupplyPda,
            kaminoProgram: kaminoProgram.programId,
            farmsProgram: KAMINO_FARM_MAINNET,
            kaminoReserveFarmState: KAMINO_RESERVE_FARM_STATE_USDC,
            collateralTokenProgram: TOKEN_PROGRAM_ID,
            liquidityTokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId, 
            associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
            instructionsSysvar: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
        })
        .signers([user])
        .rpc();
               
        console.log("withdraw signature: ", signature);
        
      });

    });

    describe.skip("Deregister and close accounts", () => {      
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
          await tokenDeregister(market, admin, mintInfo, usdcATA, programWallet.publicKey, bank, vault);

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
    });

    after(async () => {
      console.log("Defunding wallets");
      await defundWallet(admin);
      await defundWallet(user);
      console.log("Wallets defunded");
      console.log("Balance before testing: ", await connection.getBalance(programWallet.publicKey));
    });

  });

});