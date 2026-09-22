# 🍑 Testing Peach V1

This guide walks you through running tests for the **Peach V1** Anchor program across local, devnet, and mainnet environments.

### 🧪 Running Tests Locally or on Devnet

Ensure your environment is properly set up.

To build, deploy, and run tests locally or on devnet:

```bash
anchor test
```

### 🛰️ Mainnet Pyth Prices (keeper bot)

User transactions (`token_deposit`, `token_withdraw`, Kamino flows) read the
oracle stored in `Bank.oracle`. For **real Pyth prices on mainnet-beta**:

1. Register each token with `oracle` = the Pyth price-feed PDA for its mint
   (see `keeper/src/pythFeeds.ts` for feed IDs, `priceFeedPda()` in
   `keeper/src/pythUserTx.ts` to derive the address, `registerTokenWithPyth()`
   for a one-call registration). Use `max_staleness_slots: 60`.
2. Run the keeper to keep those PDAs fresh (needs `PYTH_API_KEY` — Hermes
   requires authentication since the Aug-2026 Pyth Core upgrade):
   ```bash
   cp keeper/.env.example .env  # fill in RPC_URL, keys, PYTH_API_KEY
   npm run keeper
   ```
3. For the freshest UX, bundle the price update with the user instruction via
   `depositWithLivePrices()` / `withdrawWithLivePrices()` in
   `keeper/src/pythUserTx.ts` (update PDA + Peach ix in one sequence), or pass
   the PDA directly as `oracle` and rely on the background keeper.

Full runbook: [keeper/README.md](keeper/README.md). Test helper:
`tests/helpers/pyth.ts`.

### 🌐 Mainnet Testing

#### 1. Set the Solana Cluster to Mainnet
(Replace <MAINNET_RPC_URL> in double quotes)
```bash
solana config set --url <MAINNET_RPC_URL>
```

#### 2. Generate a Program Keypair for Mainnet

```bash
solana-keygen new -o ./target/deploy/peach_v1_mainnet-keypair.json
```

#### 3. Update Program ID

Copy the new program ID into both:

* `Anchor.toml`
* `programs/peach-v1/src/lib.rs`

Change cluster to mainnet (with api-key if using custom rpc) in Anchor.toml

#### 4. Build the Program

```bash
anchor build
```

#### 5. Deploy to Mainnet

```bash
solana program deploy ./target/deploy/peach_v1.so --program-id ./target/deploy/peach_v1_mainnet-keypair.json
```

#### If Deployment Fails

You can resume the deploy using the buffer:

1. Recover the buffer keypair:

   ```bash
   solana-keygen recover --outfile keypair.json
   ```

   > You’ll be prompted to enter the seed phrase.

2. Resume deployment:

   ```bash
   solana program deploy ./target/deploy/peach_v1.so --program-id ./target/deploy/peach_v1_mainnet-keypair.json --buffer keypair.json
   ```

### ✅ Running Mainnet Tests

```bash
anchor run test
```

#### Tip: Skipping Tests

You can skip individual tests using `.skip` in your test file:

```ts
it.skip("skips this test", async () => { ... });
```

Or skip an entire block:

```ts
describe.skip("skips this block", () => { ... });
```

### 💸 Recovering SOL: Close Accounts

After testing, close the program and buffer accounts to reclaim SOL:

```bash
solana program close <PROGRAM_ID> --bypass warnings
solana program close --buffers
```
```