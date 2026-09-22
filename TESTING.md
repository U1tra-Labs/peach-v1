# Testing Peach V1

## Running Tests
Ensure your environment is set up before running tests.

```sh
anchor test
```

## Mainnet Pyth Prices (keeper bot)

User transactions (`token_deposit`, `token_withdraw`) read the oracle stored in
`Bank.oracle`. For **real Pyth prices on mainnet-beta**:

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
```