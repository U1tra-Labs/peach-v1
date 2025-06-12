import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PeachV1 } from "../../target/types/peach_v1";
import { Keypair, LAMPORTS_PER_SOL, PublicKey, Transaction } from "@solana/web3.js";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";
import { LAMPORTS_PER_SOL_FOR_TEST_WALLETS } from "./const";
import * as dotenv from 'dotenv';

export const provider = anchor.AnchorProvider.env();
anchor.setProvider(provider);
export const program = anchor.workspace.PeachV1 as Program<PeachV1>;
export const envProviderPayer = (provider.wallet as NodeWallet).payer;
import * as path from 'path';
import * as fs from 'fs';


dotenv.config();

const ENV_PATH = path.resolve(process.cwd(), ".env");


function loadEnv(): Record<string, string> {
  const envData = fs.readFileSync(ENV_PATH, 'utf-8');
  return dotenv.parse(envData); // returns an object like { KEYPAIR_0: "[...]" }
}

/**
 * Gets or creates a Keypair for the given index and updates the .env file if needed.
 * @param index number (e.g., 0, 1)
 */
function getOrCreateKeypair(index: number): Keypair {
  const envKey = `KEYPAIR_${index}`;
  // Always reload latest .env content
  const env = loadEnv();
  const keyEnv = env[envKey];

  // console.log(`Checking for Keypair at index ${index} with env key ${envKey}`);

  if (keyEnv) {
    try {
      const secretKey = Uint8Array.from(JSON.parse(keyEnv));
      // console.log(`Using existing Keypair from .env as ${envKey}`);
      return Keypair.fromSecretKey(secretKey);
    } catch (err) {
      throw new Error(`Invalid keypair format for index ${index}: ${err}`);
    }
  }

  // Keypair doesn't exist, so generate one
  const newKeypair = Keypair.generate();
  const secretArray = Array.from(newKeypair.secretKey);

  // Append to .env file
  const envLine = `\n${envKey}=${JSON.stringify(secretArray)}`;
  fs.appendFileSync(ENV_PATH, envLine);
  console.log(`Generated new Keypair and added to .env as ${envKey}`);

  return newKeypair;
}

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

export async function getFundedWallet(walletId: number): Promise<Keypair | null> {
  const wallet = getOrCreateKeypair(walletId);
  const balance = await provider.connection.getBalance(wallet.publicKey);
  if (balance < 0.5 * LAMPORTS_PER_SOL) {
    const transferSuccess = await transferSol(envProviderPayer, wallet.publicKey, LAMPORTS_PER_SOL_FOR_TEST_WALLETS * LAMPORTS_PER_SOL);
    return transferSuccess ? wallet : null;
  }
  return wallet;
}

export async function defundWallet(walletId: number) {
  const wallet = getOrCreateKeypair(walletId);
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

