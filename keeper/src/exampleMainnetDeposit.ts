/**
 * End-to-end mainnet-beta example: deposit USDC into Peach with a live Pyth price.
 *
 * Prerequisites:
 *   RPC_URL / ANCHOR_PROVIDER_URL  -> mainnet-beta RPC
 *   ANCHOR_WALLET                  -> keypair with SOL + USDC
 *   HERMES_URL / PYTH_API_KEY      -> Hermes access (API key required since Aug-2026)
 *   PYTH_SHARD_ID                  -> must match the shard used at token_register
 *                                    and by keeper/src/keeper.ts
 *   MARKET, PEACH_ACCOUNT, BANK, VAULT, USER_TOKEN_ACCOUNT env (base58)
 *
 * The token must already be registered with oracle = price-feed PDA for
 *   USDC/USD 0xeaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a
 * on the same shard. The keeper keeps that PDA fresh; this script additionally
 * bundles an update_price_feed + deposit so the deposit always consumes a
 * seconds-old price even if the background keeper is delayed.
 *
 * Run:
 *   npx ts-node keeper/src/exampleMainnetDeposit.ts
 */
import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { AnchorProvider, Program, Wallet } from "@coral-xyz/anchor";
import * as anchor from "@coral-xyz/anchor";
import fs from "fs";
import { makePythClient, depositWithLivePrices } from "./pythUserTx";
import { FEEDS } from "./pythFeeds";

async function main(): Promise<void> {
  const rpcUrl = process.env.RPC_URL ?? process.env.ANCHOR_PROVIDER_URL!;
  const walletPath = process.env.ANCHOR_WALLET!;
  const wallet = new Wallet(
    Keypair.fromSecretKey(
      Buffer.from(JSON.parse(fs.readFileSync(walletPath, "utf-8")))
    )
  );
  const connection = new Connection(rpcUrl, "confirmed");
  const provider = new AnchorProvider(connection, wallet, {
    commitment: "confirmed",
  });
  anchor.setProvider(provider);

  const peach = anchor.workspace.PeachV1 as Program;
  const pyth = makePythClient({ connection, wallet });

  const usdc = FEEDS.find((f) => f.symbol === "USDC/USD")!;
  const amount = new anchor.BN(Number(process.env.DEPOSIT_AMOUNT ?? "1000")); // native units
  const deposit = {
    amount,
    market: new PublicKey(process.env.MARKET!),
    account: new PublicKey(process.env.PEACH_ACCOUNT!),
    owner: wallet.publicKey,
    bank: new PublicKey(process.env.BANK!),
    vault: new PublicKey(process.env.VAULT!),
    priceFeedAccount: new PublicKey("11111111111111111111111111111111"), // replaced by builder
    userTokenAccount: new PublicKey(process.env.USER_TOKEN_ACCOUNT!),
    tokenAuthority: wallet.publicKey,
    signer: (wallet as Wallet).payer,
  };

  console.log(
    `[example] depositing ${amount.toString()} with live ${usdc.symbol} ${
      usdc.feedId
    }`
  );
  const sigs = await depositWithLivePrices(peach, pyth, usdc.feedId, deposit);
  console.log(`[example] landed: ${sigs.join(", ")}`);
}

if (require.main === module) {
  main().catch((err) => {
    console.error(err);
    process.exit(1);
  });
}
