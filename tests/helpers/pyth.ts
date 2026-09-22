/**
 * Test helper: thin re-exports of the keeper's Pyth client so anchor tests can
 * build mainnet-beta transactions with real prices.
 *
 * Example (mainnet):
 * ```ts
 * import { makePythClient, depositWithLivePrices } from "./pyth";
 * import { provider } from "./setup";
 * import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";
 *
 * const pyth = makePythClient({
 *   connection: provider.connection,
 *   wallet: provider.wallet as NodeWallet,
 * });
 * await depositWithLivePrices(program, pyth, SOL_FEED_ID, { ... });
 * ```
 */
export * from "../../keeper/src/pythUserTx";
export * from "../../keeper/src/pythFeeds";
