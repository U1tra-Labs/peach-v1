import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { program, provider } from "../helpers/setup";
import * as anchor from "@coral-xyz/anchor";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";

export async function tokenRegister(tokenIndex: number, mint: PublicKey, marketPDA: PublicKey, vaultPDA: PublicKey, mintInfoPDA: PublicKey, bankPDA: PublicKey, stubOracle: PublicKey, admin: Keypair) {
    const ix1 = await program.methods.tokenVaultCreate(tokenIndex)
      .accounts({
        market: marketPDA,
        admin: admin.publicKey,
        mint,
        vault: vaultPDA,
        payer: admin.publicKey, // This is the payer
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
        admin: admin.publicKey,
        mint,
        bank: bankPDA,
        vault: vaultPDA,
        mintInfo: mintInfoPDA,
        oracle: stubOracle,
        fallbackOracle: stubOracle,
        payer: admin.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      }).instruction();

    const tx = new anchor.web3.Transaction();
    tx.add(ix1);
    tx.add(ix2);

    const sig = await anchor.web3.sendAndConfirmTransaction(
      provider.connection, tx, [admin]
    );

    return sig;

}

export async function tokenDeposit(deposit_amount: anchor.BN, marketPDA: PublicKey, peachAccountPDA: PublicKey, bankPDA: PublicKey, vaultPDA: PublicKey, stubOracle: PublicKey, tokenAccount: PublicKey, user: Keypair) {    
    const tx = await program.methods
        .tokenDeposit(deposit_amount, false)
        .accounts({
        market: marketPDA,
        account: peachAccountPDA,
        owner: user.publicKey,
        bank: bankPDA,
        vault: vaultPDA,
        oracle: stubOracle,
        tokenAccount: tokenAccount,
        tokenAuthority: user.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([user])
        .rpc();
    
    return tx;
}

export async function tokenDepositIntoExisting(deposit_amount: anchor.BN, marketPDA: PublicKey, peachAccountPDA: PublicKey, bankPDA: PublicKey, vaultPDA: PublicKey, stubOracle: PublicKey, tokenAccount: PublicKey, user: Keypair) {    
    const tx = await program.methods
        .tokenDeposit(deposit_amount, false)
        .accounts({
        market: marketPDA,
        account: peachAccountPDA,
        owner: user.publicKey,
        bank: bankPDA,
        vault: vaultPDA,
        oracle: stubOracle,
        tokenAccount: tokenAccount,
        tokenAuthority: user.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([user])
        .rpc();
    
    return tx;
}

export async function tokenForceWithdraw(marketPDA: PublicKey, peachAccountPDA: PublicKey, bankPDA: PublicKey, vaultPDA: PublicKey, stubOracle: PublicKey, tokenAccount: PublicKey) {
    const envProviderPayer = (provider.wallet as NodeWallet).payer;

    const tx = await program.methods
        .tokenForceWithdraw()
        .accounts({
        market: marketPDA,
        account: peachAccountPDA,
        ownerAtaTokenAccount: tokenAccount,
        alternateOwnerTokenAccount: tokenAccount,
        bank: bankPDA,
        vault: vaultPDA,
        oracle: stubOracle,
        tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([envProviderPayer])
        .rpc();
    
    return tx;
}

export async function tokenWithdraw(withdraw_amount: anchor.BN, marketPDA: PublicKey, peachAccountPDA: PublicKey, bankPDA: PublicKey, vaultPDA: PublicKey, stubOracle: PublicKey, tokenAccount: PublicKey) {
    const envProviderPayer = (provider.wallet as NodeWallet).payer;

    const tx = await program.methods
        .tokenWithdraw(withdraw_amount, true)
        .accounts({
        market: marketPDA,
        account: peachAccountPDA,
        owner: provider.wallet.publicKey,
        bank: bankPDA,
        vault: vaultPDA,
        oracle: stubOracle,
        tokenAccount: tokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([envProviderPayer])
        .rpc();
    
    return tx;
}

export async function tokenChargeCollateralFees(marketPDA: PublicKey, peachAccountPDA: PublicKey) {
    const tx = await program.methods.tokenChargeCollateralFees()
      .accounts({
        market: marketPDA,
        account: peachAccountPDA,
      })
        .rpc();
    
    return tx;
}

export async function tokenDeregister(market: PublicKey, admin: Keypair, mintInfo: PublicKey, dustVault: PublicKey, solDestination: PublicKey, bank: PublicKey, vault: PublicKey) {
    const tx = await program.methods.tokenDeregister()
    .accounts({
      market: market,
      admin: admin.publicKey,
      mintInfo: mintInfo,
      dustVault: dustVault,
      solDestination: solDestination,
      tokenProgram: TOKEN_PROGRAM_ID
    })
    .remainingAccounts([
      { pubkey: bank, isWritable: true, isSigner: false },
      { pubkey: vault, isWritable: true, isSigner: false },
    ])
    .signers([admin])
      .rpc();
  
  return tx;
}