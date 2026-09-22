/**
 * Pyth keeper bot for Peach V1.
 *
 * What it does (Solana mainnet-beta):
 *   1. Fetches the latest signed price updates from Hermes for the configured feeds.
 *   2. Posts them to the Pyth Solana Receiver via `update_price_feed`, which advances
 *      our FIXED price-feed accounts (PDA(feed id, shard)). These are the addresses
 *      stored in `Bank.oracle` / `MintInfo.oracle` at `token_register` time.
 *   3. User transactions (`token_deposit`, `token_withdraw`, health checks) then read
 *      live prices from those fixed accounts — no per-tx Hermes round-trip needed,
 *      and the on-chain `OracleType::PythV2` parser + staleness/confidence checks apply.
 *
 * Why fixed price-feed accounts instead of ephemeral per-tx accounts?
 *   Peach constrains `bank.has_one = oracle`, i.e. the oracle account passed to
 *   deposit/withdraw MUST equal the address stored at registration. Ephemeral
 *   `PriceUpdateV2` accounts get a fresh random address every update, so they cannot
 *   satisfy `has_one`. Fixed price-feed PDAs are stable AND (per Pyth docs) parse
 *   identically to price-update accounts on-chain, so the existing
 *   `get_pyth_on_demand_state` handles them with zero program changes.
 *
 * For latency-critical flows the same Hermes payload can be bundled atomically with
 * the Peach instruction in one builder (see pythUserTx.ts): update_price_feed first,
 * Peach ix second. The keeper loop below is the background freshness guarantee.
 */
import fs from "fs";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { Wallet } from "@coral-xyz/anchor";
import { HermesClient } from "@pythnetwork/hermes-client";
import { PythSolanaReceiver } from "@pythnetwork/pyth-solana-receiver";
import { loadKeeperConfig } from "./config";
import { FEEDS, assertValidFeedIds, feedIdsFor } from "./pythFeeds";

export interface KeeperContext {
  connection: Connection;
  wallet: Wallet;
  receiver: PythSolanaReceiver;
  hermes: HermesClient;
  feedIds: string[];
  shardId: number;
  computeUnitPriceMicroLamports: number;
}

export function loadKeypair(path: string): Keypair {
  const raw = fs.readFileSync(path, "utf-8");
  return Keypair.fromSecretKey(Buffer.from(JSON.parse(raw)));
}

export function buildKeeperContext(): KeeperContext {
  const cfg = loadKeeperConfig();
  const connection = new Connection(cfg.rpcUrl, "confirmed");
  const wallet = new Wallet(loadKeypair(cfg.keeperKeypairPath));
  // PythSolanaReceiver builds its own AnchorProvider from connection+wallet
  // and exposes it as `receiver.provider` for sendAll.
  const receiver = new PythSolanaReceiver({ connection, wallet });
  const hermes = new HermesClient(
    cfg.hermesUrl,
    cfg.pythApiKey ? { accessToken: cfg.pythApiKey } : undefined
  );
  const feedIds = feedIdsFor(
    cfg.feedSymbols.length > 0 ? cfg.feedSymbols : undefined
  );
  if (feedIds.length === 0) {
    throw new Error(
      `No feeds selected. Known symbols: ${FEEDS.map((f) => f.symbol).join(
        ", "
      )}`
    );
  }
  assertValidFeedIds(feedIds);
  return {
    connection,
    wallet,
    receiver,
    hermes,
    feedIds,
    shardId: cfg.shardId,
    computeUnitPriceMicroLamports: cfg.computeUnitPriceMicroLamports,
  };
}

/** Derive the fixed price-feed account address for a feed id + shard. */
export function priceFeedAccountAddress(
  ctx: KeeperContext,
  feedId: string,
  shardId?: number
): PublicKey {
  return ctx.receiver.getPriceFeedAccountAddress(
    shardId ?? ctx.shardId,
    feedId
  );
}

/**
 * One keeper crank: fetch Hermes updates and advance our price-feed accounts.
 * Returns the tx signatures landed + best-effort parsed prices for logging.
 */
export async function crankOnce(ctx: KeeperContext): Promise<{
  signatures: string[];
  publishTimes: Record<string, number>;
}> {
  // Single Hermes round-trip: binary payload for on-chain + parsed for logs.
  const priceUpdates = await ctx.hermes.getLatestPriceUpdates(ctx.feedIds, {
    encoding: "base64",
    parsed: true,
  });
  const priceUpdateData = priceUpdates.binary.data;

  const builder = ctx.receiver.newTransactionBuilder({});
  await builder.addUpdatePriceFeed(priceUpdateData, ctx.shardId);
  const txs = await builder.buildVersionedTransactions({
    computeUnitPriceMicroLamports: ctx.computeUnitPriceMicroLamports,
  });
  // AnchorProvider.sendAll signs with the keeper wallet + ephemeral signers,
  // sends, and confirms. This is the SDK-recommended dispatch path.
  const signatures: string[] = await ctx.receiver.provider.sendAll(txs, {
    skipPreflight: true,
  } as any);

  const publishTimes: Record<string, number> = {};
  for (const feed of priceUpdates.parsed ?? []) {
    publishTimes[feed.id] = feed.price.publish_time;
  }

  return { signatures, publishTimes };
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function main(): Promise<void> {
  const cfg = loadKeeperConfig();
  const ctx = buildKeeperContext();
  console.log(
    `[pyth-keeper] RPC=${cfg.rpcUrl} Hermes=${cfg.hermesUrl} shard=${
      ctx.shardId
    } feeds=${ctx.feedIds.join(",")}`
  );
  for (const feedId of ctx.feedIds) {
    console.log(
      `[pyth-keeper] ${feedId} -> ${priceFeedAccountAddress(
        ctx,
        feedId
      ).toBase58()}`
    );
  }

  // eslint-disable-next-line no-constant-condition
  while (true) {
    try {
      const { signatures } = await crankOnce(ctx);
      console.log(
        `[pyth-keeper] ${new Date().toISOString()} updated ${
          ctx.feedIds.length
        } feed(s): ${signatures.join(", ")}`
      );
    } catch (err: any) {
      console.error(`[pyth-keeper] crank failed: ${err?.message ?? err}`);
    }
    await sleep(cfg.intervalMs);
  }
}

if (require.main === module) {
  main().catch((err) => {
    console.error(err);
    process.exit(1);
  });
}
