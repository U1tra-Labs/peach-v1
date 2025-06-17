import { Keypair, PublicKey, SystemProgram, Transaction, TransactionInstruction } from "@solana/web3.js";
import { User } from "../../objects/user";
import { program, provider } from "../../helpers/setup";
import * as anchor from "@coral-xyz/anchor";
import { KAMINO_FARM_MAINNET, KAMINO_LENDING, KAMINO_LENDING_MAIN_MARKET, KAMINO_LENDING_MAIN_MARKET_AUTHORITY, KAMINO_RESERVE_LIQUIDITY_FEE_VAULT_PYUSD } from "../../helpers/const";
import { Token } from "../../objects/token";
import { ASSOCIATED_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";

export async function kaminoDeposit(deposit_amount: anchor.BN, user: User, token: Token, market: PublicKey, oracle: PublicKey, preIx: TransactionInstruction[], active_positions: Token[]) {
    const remainingAccounts: { pubkey: PublicKey; isWritable: boolean; isSigner: boolean }[] = [];
    if (active_positions.length > 0) {
        for (const position of active_positions) {
        remainingAccounts.push({
            pubkey: position.bank[0], // Use the first bank from the token object
            isWritable: true,
            isSigner: false,
        });
        remainingAccounts.push({
            pubkey: oracle, // Use the first vault from the token object
            isWritable: true,
            isSigner: false,
        });
        }
    } 

    const programIx = await program.methods.kaminoDeposit(
          deposit_amount,
          false,
        ).accounts({
          market: market,
          peachAccount: user.peachAccount,
          bank: token.bank[0],
          oracle: oracle,

          owner: user.wallet.publicKey,
          obligation: user.obligation,
          lendingMarket: KAMINO_LENDING_MAIN_MARKET,
          lendingMarketAuthority: KAMINO_LENDING_MAIN_MARKET_AUTHORITY,
          reserve: token.reserve,
          reserveLiquidityMint: token.reserve_liquidity_mint,
          reserveLiquiditySupply: token.reserve_liquidity_supply,
          reserveCollateralMint: token.reserve_collateral_mint,
        //   reserveCollateralMint: token.reserve_liquidity_mint, // devnet
          reserveDestinationDepositCollateral: token.reserve_collateral_supply,
          userSourceLiquidity: user.getTokenAccount(token.tokenIndex),
          kaminoProgram: KAMINO_LENDING,
          collateralTokenProgram: token.programId,
          liquidityTokenProgram: token.programId,
          instructionSysvarAccount: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,

          obligationFarmUserState: user.getObligationFarmForToken(token.tokenIndex).colalteralFarm,
          reserveFarmState: token.kaminReserveFarmStateCollateral,

          farmsProgram: KAMINO_FARM_MAINNET,
          
          systemProgram: SystemProgram.programId,
          associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
        })
        .remainingAccounts(remainingAccounts)
        .instruction();

    preIx.push(programIx);
    
    const signature = await sendKaminoLendingTransaction(user.wallet, preIx);
    console.log("Kamino Deposit:", signature);
    return signature;
}

export async function kaminoWithdraw(withdraw_amount: anchor.BN, user: User, token: Token, market: PublicKey, oracle: PublicKey, preIx: TransactionInstruction[], active_positions: Token[]) {
    const remainingAccounts: { pubkey: PublicKey; isWritable: boolean; isSigner: boolean }[] = [];
    if (active_positions.length > 0) {
        for (const position of active_positions) {
        remainingAccounts.push({
            pubkey: position.bank[0], // Use the first bank from the token object
            isWritable: true,
            isSigner: false,
        });
        remainingAccounts.push({
            pubkey: oracle, // Use the first vault from the token object
            isWritable: true,
            isSigner: false,
        });
        }
    } 

    const programIx = await program.methods.kaminoWithdraw(
        withdraw_amount
    ).accounts({
        market: market,
        peachAccount: user.peachAccount,
        bank:  token.bank[0],
        oracle: oracle,
        owner: user.wallet.publicKey,
        obligation: user.obligation,
        lendingMarket: KAMINO_LENDING_MAIN_MARKET,
        lendingMarketAuthority: KAMINO_LENDING_MAIN_MARKET_AUTHORITY,
        withdrawReserve: token.reserve,
        reserveLiquidityMint: token.reserve_liquidity_mint,
        reserveSourceCollateral: token.reserve_collateral_supply,
        reserveCollateralMint: token.reserve_collateral_mint,
        // reserveCollateralMint: token.reserve_liquidity_mint, // devnet
        reserveLiquiditySupply: token.reserve_liquidity_supply,
        // reserveLiquiditySupply: token.reserve, // devnet
        userDestinationLiquidity: user.getTokenAccount(token.tokenIndex),
        kaminoProgram: KAMINO_LENDING,
        collateralTokenProgram: token.programId,
        liquidityTokenProgram: token.programId,
        instructionSysvarAccount: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
        
        obligationFarmUserState: user.getObligationFarmForToken(token.tokenIndex).colalteralFarm,
        reserveFarmState: token.kaminReserveFarmStateCollateral,

        farmsProgram: KAMINO_FARM_MAINNET,
        
        systemProgram: SystemProgram.programId,
        associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
    })
        .remainingAccounts(remainingAccounts)
        .instruction();
    
    const signature = await sendKaminoLendingTransaction(user.wallet, preIx.concat(programIx));
    console.log("Kamino withdraw:", signature);
    return signature;
}

export async function kaminoBorrow(borrow_amount: anchor.BN, user: User, token: Token, market: PublicKey, oracle: PublicKey, preIx: TransactionInstruction[], pyusdATA: PublicKey, active_positions: Token[]) {
    const remainingAccounts: { pubkey: PublicKey; isWritable: boolean; isSigner: boolean }[] = [];
    if (active_positions.length > 0) {
        for (const position of active_positions) {
        remainingAccounts.push({
            pubkey: position.bank[0], // Use the first bank from the token object
            isWritable: true,
            isSigner: false,
        });
        remainingAccounts.push({
            pubkey: oracle, // Use the first vault from the token object
            isWritable: true,
            isSigner: false,
        });
        }
    } 

    const programIx = await program.methods.kaminoBorrow(
        borrow_amount
    ).accounts({
        market: market,
        peachAccount: user.peachAccount,
        bank: token.bank[0],
        oracle: oracle,
        owner:  user.wallet.publicKey,
        obligation: user.obligation,
        lendingMarket: KAMINO_LENDING_MAIN_MARKET,
        lendingMarketAuthority: KAMINO_LENDING_MAIN_MARKET_AUTHORITY,
        borrowReserve: token.reserve,
        borrowReserveLiquidityMint: token.reserve_liquidity_mint,
        reserveSourceLiquidity: token.reserve_liquidity_supply, 
        // reserveSourceLiquidity: pyusdATA, // devnet
        borrowReserveLiquidityFeeReceiver: KAMINO_RESERVE_LIQUIDITY_FEE_VAULT_PYUSD, 
        // borrowReserveLiquidityFeeReceiver: pyusdATA, // devnet
        userDestinationLiquidity: user.getTokenAccount(token.tokenIndex),
        tokenProgram: token.programId,
        instructionSysvarAccount: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
        obligationFarmUserState: user.getObligationFarmForToken(token.tokenIndex).debtFarm,
        reserveFarmState: token.kaminReserveFarmStateDebt,
        farmsProgram: KAMINO_FARM_MAINNET,
        kaminoProgram: KAMINO_LENDING,
        systemProgram: SystemProgram.programId,
        associatedTokenProgram: ASSOCIATED_PROGRAM_ID, 
    })
    .remainingAccounts(remainingAccounts)
    .instruction();
    
    const signature = await sendKaminoLendingTransaction(user.wallet, preIx.concat(programIx));
    console.log("Kamino borrow:", signature);
    return signature;
}

export async function kaminoRepay(repay_amount: anchor.BN, user: User, token: Token, market: PublicKey, oracle: PublicKey, preIx: TransactionInstruction[], pyusdATA: PublicKey, active_positions: Token[]) {
    const remainingAccounts: { pubkey: PublicKey; isWritable: boolean; isSigner: boolean }[] = [];
    if (active_positions.length > 0) {
        for (const position of active_positions) {
        remainingAccounts.push({
            pubkey: position.bank[0], // Use the first bank from the token object
            isWritable: true,
            isSigner: false,
        });
        remainingAccounts.push({
            pubkey: oracle, // Use the first vault from the token object
            isWritable: true,
            isSigner: false,
        });
        }
    } 

    const programIx = await program.methods.kaminoRepay(
        repay_amount
    ).accounts({
        market: market,
        peachAccount: user.peachAccount,
        bank: token.bank[0],
        oracle: oracle,
        owner: user.wallet.publicKey,
        obligation: user.obligation,
        lendingMarket: KAMINO_LENDING_MAIN_MARKET,
        repayReserve: token.reserve, // KAMINO_RESERVE_PYUSD_STATE, // check out
        reserveLiquidityMint: token.reserve_liquidity_mint,
        reserveDestinationLiquidity: token.reserve_liquidity_supply,
        // reserveDestinationLiquidity: pyusdATA, // devnet
        userSourceLiquidity: user.getTokenAccount(token.tokenIndex), // pyusd ATA
        tokenProgram: token.programId,
        instructionSysvarAccount: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
        
        obligationFarmUserState: user.getObligationFarmForToken(token.tokenIndex).debtFarm,
        reserveFarmState: token.kaminReserveFarmStateDebt, // PYUSD

        lendingMarketAuthority: KAMINO_LENDING_MAIN_MARKET_AUTHORITY,

        farmsProgram: KAMINO_FARM_MAINNET,

        kaminoProgram: KAMINO_LENDING,

        systemProgram: SystemProgram.programId,
        associatedTokenProgram: ASSOCIATED_PROGRAM_ID, 
    })
    .remainingAccounts(remainingAccounts)
    .instruction();
    
    const signature = await sendKaminoLendingTransaction(user.wallet, preIx.concat(programIx));
    console.log("Kamino repay:", signature);
    return signature;
}

async function sendKaminoLendingTransaction(user: Keypair, instructions: anchor.web3.TransactionInstruction[]) {
    const blockhashWithContext = await provider.connection.getLatestBlockhash();

    const Tx = new Transaction({
        feePayer: user.publicKey,
        blockhash: blockhashWithContext.blockhash,
        lastValidBlockHeight: blockhashWithContext.lastValidBlockHeight,
    }).add(...instructions);

    const signature = await provider.connection.sendTransaction(
        Tx,
        [user],
        { skipPreflight: true }
    )   

    return signature;
}