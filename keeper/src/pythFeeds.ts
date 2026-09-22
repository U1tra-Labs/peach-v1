/**
 * Pyth feed registry for Peach V1.
 *
 * Peach banks store `bank.oracle` as a fixed account address (Anchor `has_one = oracle`
 * constraint on deposit/withdraw). For Pyth pull oracles that means the bank must point
 * at a *fixed* price-feed account (PDA derived from feed id + shard), NOT at an ephemeral
 * per-tx price-update account. The keeper in `keeper.ts` keeps those fixed accounts fresh
 * via `update_price_feed`, and user transactions simply pass the fixed PDA as `oracle`.
 *
 * Feed IDs are the Pyth Core (pull) hex IDs. Full list:
 * https://docs.pyth.network/price-feeds/core/price-feeds/price-feed-ids
 *
 * Well-known mainnet pull feed IDs:
 * - SOL/USD:  0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d
 * - BTC/USD:  0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43
 * - ETH/USD:  0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace
 * - USDC/USD: 0xeaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a
 * - USDT/USD: 0x2b89b9dc8fdf9f34709a5b106b472f0f39bb6ca9ce04b0fd7f2e971688e2e53b
 *
 * Legacy push (v1) oracle accounts still referenced on-chain for reference only:
 * - SOL push:  H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG
 * - USDC push: Gnt27xtC473ZT2Mw5u8wZ68Z3gULkSTb5DuxJy7eJotD
 */

export const PYTH_RECEIVER_PROGRAM_ID =
  "rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ"; // mainnet-beta + devnet

export const PYTH_PUSH_ORACLE_PROGRAM_ID =
  "pythWSnswVUd12oZpeFP8e9CVaEqJg25g1Vtc2biRsT";

export interface FeedEntry {
  symbol: string;
  /** 0x-prefixed 32-byte hex feed id */
  feedId: string;
  /** token mint this feed prices (null when N/A, e.g. BTC has no Solana mint) */
  mint: string | null;
  /** native decimals used by Peach `Bank.mint_decimals` for price scaling */
  mintDecimals: number;
}

export const FEEDS: FeedEntry[] = [
  {
    symbol: "SOL/USD",
    feedId:
      "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d",
    mint: "So11111111111111111111111111111111111111112",
    mintDecimals: 9,
  },
  {
    symbol: "USDC/USD",
    feedId:
      "0xeaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a",
    mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    mintDecimals: 6,
  },
  {
    symbol: "BTC/USD",
    feedId:
      "0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43",
    mint: null,
    mintDecimals: 8,
  },
  {
    symbol: "ETH/USD",
    feedId:
      "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace",
    mint: null,
    mintDecimals: 8,
  },
  {
    symbol: "USDT/USD",
    feedId:
      "0x2b89b9dc8fdf9f34709a5b106b472f0f39bb6ca9ce04b0fd7f2e971688e2e53b",
    mint: "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",
    mintDecimals: 6,
  },
];

export function feedIdsFor(symbols?: string[]): string[] {
  if (!symbols || symbols.length === 0) return FEEDS.map((f) => f.feedId);
  const wanted = new Set(symbols.map((s) => s.toUpperCase()));
  return FEEDS.filter((f) => wanted.has(f.symbol.toUpperCase())).map(
    (f) => f.feedId
  );
}

/** Feed IDs must be 0x + 64 hex chars (32 bytes). Fails fast on typos. */
export function assertValidFeedId(feedId: string): void {
  if (!/^0x[0-9a-fA-F]{64}$/.test(feedId)) {
    throw new Error(
      `Invalid Pyth feed id ${feedId}: expected 0x followed by 64 hex chars`
    );
  }
}

export function assertValidFeedIds(feedIds: string[]): void {
  for (const id of feedIds) assertValidFeedId(id);
}

export function feedForMint(mint: string): FeedEntry | undefined {
  return FEEDS.find((f) => f.mint === mint);
}

export function feedForSymbol(symbol: string): FeedEntry | undefined {
  return FEEDS.find((f) => f.symbol.toUpperCase() === symbol.toUpperCase());
}
