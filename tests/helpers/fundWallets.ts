import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PeachV1 } from "../../target/types/peach_v1";
import { Keypair, LAMPORTS_PER_SOL, PublicKey, Transaction } from "@solana/web3.js";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";
import { LAMPORTS_PER_SOL_FOR_TEST_WALLETS } from "./const";

export const provider = anchor.AnchorProvider.env();
anchor.setProvider(provider);
export const program = anchor.workspace.PeachV1 as Program<PeachV1>;
export const envProviderPayer = (provider.wallet as NodeWallet).payer;

export async function transferSol(sender: Keypair, receiver: PublicKey, amount: number) {
    try {
        const instruction = anchor.web3.SystemProgram.transfer({
            fromPubkey: sender.publicKey,
            toPubkey: receiver,
            lamports: amount,
        });
        const signature = await provider.connection.sendTransaction(new Transaction().add(instruction), [sender]);
        await provider.connection.confirmTransaction(signature);
    } catch (e) {
        console.log(`Transfer failed: ${e}`);
        return false;
    }
    return true;
}

export async function getFundedWallet() {
    const wallet = new Keypair();
    const transferSuccess = await transferSol(envProviderPayer, wallet.publicKey, LAMPORTS_PER_SOL_FOR_TEST_WALLETS * LAMPORTS_PER_SOL);
    return transferSuccess ? wallet : null;
}

export async function defundWallet(wallet: Keypair) {
  try {
    const balance = await provider.connection.getBalance(wallet.publicKey);
    if (balance > 0) {
        await transferSol(wallet, envProviderPayer.publicKey, balance-5000); // 5000 lamports for gas fees
    }
  }
  catch (e) {
    console.log(`Defunding wallet ${wallet.publicKey} failed ` + e);
  }
 
}