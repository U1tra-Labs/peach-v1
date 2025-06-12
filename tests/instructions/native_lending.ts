import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { program, provider } from "../helpers/setup";
import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";
import { Token } from "../objects/token";
import { token } from "@coral-xyz/anchor/dist/cjs/utils";
import { User } from "../objects/user";

export async function tokenDeposit(deposit_amount: anchor.BN, marketPDA: PublicKey, user: User, token: Token, oracle: PublicKey, is_repay: boolean) {    
    
  const remainingAccounts: { pubkey: PublicKey; isWritable: boolean; isSigner: boolean }[] = [];

  remainingAccounts.push({
    pubkey: token.bank[0], // Use the first token account from the user's token accounts
    isWritable: true,
    isSigner: false,
  });
  remainingAccounts.push({
    pubkey: oracle, // Use the first token account from the user's token accounts
    isWritable: true,
    isSigner: false,
  });

  const tx = await program.methods
        .tokenDeposit(deposit_amount, false)
        .accounts({
        market: marketPDA,
        account: user.peachAccount,
        owner: user.wallet.publicKey,
        bank: token.bank[0], // Use the first bank from the token object
        vault: token.vault[0], // Use the first vault from the token object
        oracle: oracle,
        tokenAccount: user.tokenAccounts[token.tokenIndex-1], // Use the first token account from the user's token accounts
        tokenAuthority: user.wallet.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        })
        .remainingAccounts(remainingAccounts)
        .signers([user.wallet])
        .rpc();
    
    return tx;
}

export async function tokenDepositIntoExisting(deposit_amount: anchor.BN, marketPDA: PublicKey, user: User, token: Token, oracle: PublicKey) {    
  const remainingAccounts: { pubkey: PublicKey; isWritable: boolean; isSigner: boolean }[] = [];
  remainingAccounts.push({
    pubkey: token.bank[0], // Use the first token account from the user's token accounts
    isWritable: true,
    isSigner: false,
  });
  remainingAccounts.push({
    pubkey: oracle, // Use the first token account from the user's token accounts
    isWritable: true,
    isSigner: false,
  });

  const tx = await program.methods
      .tokenDeposit(deposit_amount, false)
      .accounts({
      market: marketPDA,
      account: user.peachAccount,
      owner: user.wallet.publicKey,
      bank: token.bank[0], // Use the first bank from the token object
      vault: token.vault[0], // Use the first vault from the token object
      oracle: oracle,
      tokenAccount: user.tokenAccounts[token.tokenIndex-1], 
      tokenAuthority: user.wallet.publicKey,
      tokenProgram: TOKEN_PROGRAM_ID,
      })
    .remainingAccounts(remainingAccounts)
      .signers([user.wallet])
      .rpc();
  
  return tx;
}

export async function tokenForceWithdraw(marketPDA: PublicKey, user: User, token: Token, oracle: PublicKey) {
    const envProviderPayer = (provider.wallet as NodeWallet).payer;

    const tx = await program.methods
        .tokenForceWithdraw()
        .accounts({
        market: marketPDA,
        account: user.peachAccount,
        ownerAtaTokenAccount: user.tokenAccounts[token.tokenIndex-1],
        alternateOwnerTokenAccount: user.tokenAccounts[token.tokenIndex-1],
        bank: token.bank[0], // Use the first bank from the token object
        vault: token.vault[0], // Use the first vault from the token object
        oracle: oracle,
        tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([envProviderPayer])
        .rpc();
    
    return tx;
}

export async function tokenWithdraw(withdraw_amount: anchor.BN, marketPDA: PublicKey, user: User, token: Token, oracle: PublicKey, is_borrw: boolean) {
  const envProviderPayer = (provider.wallet as NodeWallet).payer;
  
  let remainingAccounts: { pubkey: PublicKey; isWritable: boolean; isSigner: boolean }[] = [];
  if(is_borrw) {
    // If is_borrw is true, we need to add the user's token account as a writable account
    remainingAccounts.push({
      pubkey: token.bank[0], // Use the first token account from the user's token accounts
      isWritable: true,
      isSigner: false,
    });
    remainingAccounts.push({
      pubkey: oracle, // Use the first token account from the user's token accounts
      isWritable: true,
      isSigner: false,
    });
  }

    const tx = await program.methods
        .tokenWithdraw(withdraw_amount, true)
        .accounts({
        market: marketPDA,
        account: user.peachAccount,
        owner: user.wallet.publicKey,
        bank: token.bank[0], // Use the first bank from the token object
        vault: token.vault[0], // Use the first vault from the token object
        oracle: oracle,
        tokenAccount: user.tokenAccounts[token.tokenIndex-1], // Use the first token account from the user's token accounts
        tokenProgram: TOKEN_PROGRAM_ID,
        })
        .remainingAccounts(remainingAccounts)
        .signers([user.wallet])
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
