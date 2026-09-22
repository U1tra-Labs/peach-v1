# Pyth Keeper Bot — Peach V1 (Solana mainnet-beta)

Background process that posts **real Pyth pull-oracle prices** so Peach user
transactions (`token_deposit`, `token_withdraw`, health checks) read live prices
on **Solana mainnet-beta**.

## How it works

Peach stores `Bank.oracle` as a fixed address (`has_one = oracle` on
deposit/withdraw). For Pyth pull that address must be a **fixed price-feed PDA**
`PDA(feed id, shard)` owned by the Pyth receiver
`rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ` — not an ephemeral per-tx account.
On-chain `determine_oracle_type` maps receiver-owned accounts to
`OracleType::PythV2`, `get_pyth_on_demand_state` requires
`VerificationLevel::Full`, and `Bank::oracle_price` enforces the
`conf_filter` / `max_staleness_slots` gates. No program change was needed.

The keeper loop (`src/keeper.ts`):

1. `HermesClient.getLatestPriceUpdates(feedIds)` → signed base64 payloads.
2. `PythSolanaReceiver.addUpdatePriceFeed(payload, shardId)` → advances our fixed
   PDAs on-chain (fully verified, multi-tx under the hood).
3. Repeat every `KEEPER_INTERVAL_MS` (default 10s — well under the 60-slot /
   ~24s staleness window used at `token_register`).

User flows (`src/pythUserTx.ts`):

- **Background mode:** keeper keeps PDAs fresh; deposits pass the PDA as `oracle`.
- **Bundled mode (recommended for UX):** `depositWithLivePrices` /
  `withdrawWithLivePrices` fetch Hermes in the client and build one Pyth
  transaction-builder sequence `[update_price_feed(PDA)…, peach_ix(oracle=PDA)]`,
  so the Peach instruction always consumes a seconds-old price even if the
  background keeper hiccups.

## Setup (mainnet-beta)

```bash
npm install
export RPC_URL="https://api.mainnet-beta.solana.com"   # or private RPC
export ANCHOR_WALLET=~/.config/solana/id.json
export KEEPER_KEYPAIR=~/.config/solana/id.json
export HERMES_URL="https://hermes.pyth.network"         # or https://pyth.dourolabs.app/hermes
export PYTH_API_KEY="<your-pyth-api-key>"               # required since Aug-2026 upgrade
export PYTH_SHARD_ID="0"
export PYTH_FEEDS="SOL/USD,USDC/USD"                    # default: all feeds in src/pythFeeds.ts
export KEEPER_INTERVAL_MS="10000"
```

Fund the keeper wallet with SOL (each crank is 1–3 txs).

## 1. Register tokens against Pyth PDAs

Derive the PDA for each mint (or run the keeper once and read its log lines):

```ts
priceFeedPda(client, "0xef0d8b6...56d") // SOL/USD, shard 0
```

Then `token_register` with:

- `oracle = priceFeedPDA`, `fallbackOracle = priceFeedPDA` (or a stub as fallback)
- `conf_filter: 0.1`, `max_staleness_slots: 60`

Helper: `registerTokenWithPyth()` in `src/pythUserTx.ts`.

## 2. Run the keeper

```bash
npx ts-node keeper/src/keeper.ts
# or: npm run keeper
```

Expected log:

```
[pyth-keeper] RPC=... shard=0 feeds=0xef0d...,0xeaa0...
[pyth-keeper] 0xef0d... -> <price-feed PDA>
[pyth-keeper] 2026-... updated 2 feed(s): <sig>, ...
```

## 3. User transactions with live prices

Background mode — pass the PDA directly (see `tests/instructions/token.ts`,
replacing `stubOracle` with the PDA):

```ts
.tokenDeposit(amount, false).accounts({ ..., oracle: solPriceFeedPda, ... })
```

Bundled mode — update + consume in one sequence:

```ts
import { makePythClient, depositWithLivePrices } from "./keeper/src/pythUserTx";
const sigs = await depositWithLivePrices(peach, pyth, SOL_FEED_ID, { ... });
```

Full example: `src/exampleMainnetDeposit.ts`.

## Feeds

See `src/pythFeeds.ts`. Defaults: SOL/USD, USDC/USD, BTC/USD, ETH/USD, USDT/USD.
Add more from https://docs.pyth.network/price-feeds/core/price-feeds/price-feed-ids
and set `PYTH_FEEDS` accordingly. The keeper + PDA derivation are feed-agnostic.

## Notes

- Keeper posts **fully verified** updates only. Never switch the keeper to
  `addPostPartiallyVerifiedPriceUpdates` — on-chain requires `VerificationLevel::Full`.
- Keep `KEEPER_INTERVAL_MS` comfortably below `max_staleness_slots * 400ms`.
- Shard choice is an app-level isolation knob (u16). Shard 0 is co-sponsored by the
  Pyth Data Association; a dedicated shard (e.g. 9) isolates you from others'
  congestion. Whatever you pick must match at register, keeper, and client.
