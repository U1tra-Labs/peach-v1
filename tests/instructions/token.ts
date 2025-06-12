import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { program, provider } from "../helpers/setup";
import * as anchor from "@coral-xyz/anchor";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";
import { Token } from "../objects/token";
import { token } from "@coral-xyz/anchor/dist/cjs/utils";
import { User } from "../objects/user";
import { TokenRegisterParams } from "../objects/token_register_params";

export async function tokenRegister(
  marketPDA: PublicKey,
  token: Token,
  oracle: PublicKey,
  admin: Keypair,
  params: TokenRegisterParams
) {
  const ix1 = await program.methods.tokenVaultCreate(token.tokenIndex)
    .accounts({
      market: marketPDA,
      admin: admin.publicKey,
      mint: token.mint,
      vault: token.vault[0],
      payer: admin.publicKey,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
      rent: anchor.web3.SYSVAR_RENT_PUBKEY,
    })
    .instruction();

    const ix2 = await program.methods.tokenRegister(
      token.tokenIndex,
      token.name,
      params.oracleConfig,
      params.interestRateParams,
      params.loanFeeRate,
      params.loanOriginationFeeRate,
      params.maintAssetWeight,
      params.initAssetWeight,
      params.maintLiabWeight,
      params.initLiabWeight,
      params.liquidationFee,
      params.stablePriceDelayIntervalSeconds,
      params.stablePriceDelayGrowthLimit,
      params.stablePriceGrowthLimit,
      params.minVaultToDepositsRatio,
      params.netBorrowLimitPerWindowQuote,
      params.netBorrowLimitWindowSizeTs,
      params.borrowWeightScaleStartQuote,
      params.depositWeightScaleStartQuote,
      params.reduceOnly,
      params.interestCurveScaling,
      params.interestTargetUtilization,
      params.depositLimit,
      params.zeroUtilRate,
      params.platformLiquidationFee,
      params.disableAssetLiquidation,
      params.collateralFeePerDay,
      params.tier
    )
    .accounts({
      market: marketPDA,
      admin: admin.publicKey,
      mint: token.mint,
      bank: token.bank[0],
      vault: token.vault[0],
      mintInfo: token.mint_info,
      oracle: oracle,
      fallbackOracle: oracle,
      payer: admin.publicKey,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
      rent: anchor.web3.SYSVAR_RENT_PUBKEY,
    })
    .instruction();

  const tx = new anchor.web3.Transaction();
  tx.add(ix1);
  tx.add(ix2);

  const sig = await anchor.web3.sendAndConfirmTransaction(
    provider.connection, tx, [admin]
  );

  return sig;
}

export async function tokenAddBank(token: Token, market: PublicKey, admin: Keypair) {
  const tx = await program.methods.tokenAddBank(token.tokenIndex, 1)
    .accounts({
      market: market,
      admin: admin.publicKey,
      mint: token.mint,
      existingBank: token.bank[0], // Use the first bank from the token object
      bank: token.bank[1], // Use the first bank from the token object
      vault: token.vault[1], // Use the first vault from the token object  
      mintInfo: token.mint_info,
      payer: admin.publicKey,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
      rent: anchor.web3.SYSVAR_RENT_PUBKEY,
    })
    .signers([admin])
    .rpc();
  
  return tx;
}

export async function tokenDeregister(market: PublicKey, admin: Keypair, token: Token, dustVault: PublicKey, solDestination: PublicKey) {

  if(token.bank.length !== token.vault.length) {
    throw new Error("Bank and vault arrays must be of the same length");
  }

  const remainingAccounts: { pubkey: PublicKey; isWritable: boolean; isSigner: boolean }[] = [];

  const len = token.bank.length; // bank and vault arrays are of the same length

  for (let i = 0; i < len; i++) {
    if (token.bank[i]) remainingAccounts.push({ pubkey: token.bank[i], isWritable: true, isSigner: false });
    if (token.vault[i]) remainingAccounts.push({ pubkey: token.vault[i], isWritable: true, isSigner: false });
  }

  const tx = await program.methods.tokenDeregister()
    .accounts({
      market: market,
      admin: admin.publicKey,
      mintInfo: token.mint_info,
      dustVault: dustVault,
      solDestination: solDestination,
      tokenProgram: TOKEN_PROGRAM_ID
    })
    .remainingAccounts(remainingAccounts)
    .signers([admin])
    .rpc();

  return tx;
}