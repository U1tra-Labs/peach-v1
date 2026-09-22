import dotenv from "dotenv";
dotenv.config();

/**
 * Keeper runtime config. Everything is env-overridable so the same code runs
 * unchanged on devnet and on Solana mainnet-beta.
 *
 * Required on mainnet-beta:
 *   RPC_URL               e.g. https://api.mainnet-beta.solana.com (or a private RPC)
 *   KEEPER_KEYPAIR        path to solana keypair json (also respects ANCHOR_WALLET)
 *
 * Pyth/Hermes (Hermes requires an API key since the Aug-2026 Pyth Core upgrade):
 *   HERMES_URL            default https://hermes.pyth.network (stable). The upgraded
 *                         backend https://pyth.dourolabs.app/hermes is a drop-in
 *                         replacement — set it if your key was issued there.
 *   PYTH_API_KEY          Bearer token for Hermes. If unset, Hermes calls are made
 *                         without auth and will fail on the public endpoint.
 *
 * Keeper behavior:
 *   PYTH_SHARD_ID         u16 shard for our price-feed accounts (default 0).
 *                         Shard 0 is also sponsored by the Pyth Data Association, but
 *                         running our own keeper on any shard (incl. 0) guarantees
 *                         freshness for Peach staleness checks. Use a dedicated shard
 *                         (e.g. 9) to isolate from other apps' congestion if desired.
 *   PYTH_FEEDS            comma-separated symbols to maintain, e.g. "SOL/USD,USDC/USD".
 *                         Default: all feeds in pythFeeds.ts.
 *   KEEPER_INTERVAL_MS    poll interval (default 10000). Keep well under
 *                         `max_staleness_slots` (~60 slots ≈ 24s) used at token_register.
 *   COMPUTE_UNIT_PRICE    micro-lamports for pyth + peach txs (default 50000).
 */

export interface KeeperConfig {
  rpcUrl: string;
  keeperKeypairPath: string;
  hermesUrl: string;
  pythApiKey?: string;
  shardId: number;
  feedSymbols: string[];
  intervalMs: number;
  computeUnitPriceMicroLamports: number;
}

export function loadKeeperConfig(): KeeperConfig {
  const rpcUrl =
    process.env.RPC_URL ??
    process.env.ANCHOR_PROVIDER_URL ??
    "https://api.devnet.solana.com";
  const keeperKeypairPath =
    process.env.KEEPER_KEYPAIR ??
    process.env.ANCHOR_WALLET ??
    `${process.env.HOME ?? "~"}/.config/solana/id.json`;
  const hermesUrl = process.env.HERMES_URL ?? "https://hermes.pyth.network";
  const pythApiKey = process.env.PYTH_API_KEY;
  const shardId = Number(process.env.PYTH_SHARD_ID ?? "0");
  const feedSymbols = (process.env.PYTH_FEEDS ?? "")
    .split(",")
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
  const intervalMs = Number(process.env.KEEPER_INTERVAL_MS ?? "10000");
  const computeUnitPriceMicroLamports = Number(
    process.env.COMPUTE_UNIT_PRICE ?? "50000"
  );

  if (!Number.isInteger(shardId) || shardId < 0 || shardId > 65535) {
    throw new Error(
      `PYTH_SHARD_ID must be a u16, got ${process.env.PYTH_SHARD_ID}`
    );
  }

  return {
    rpcUrl,
    keeperKeypairPath,
    hermesUrl,
    pythApiKey,
    shardId,
    feedSymbols,
    intervalMs,
    computeUnitPriceMicroLamports,
  };
}
