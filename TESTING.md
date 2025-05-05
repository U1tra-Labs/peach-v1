# 🍑 Testing Peach V1

This guide walks you through running tests for the **Peach V1** Anchor program across local, devnet, and mainnet environments.

### 🧪 Running Tests Locally or on Devnet

Ensure your environment is properly set up.

To build, deploy, and run tests locally or on devnet:

```bash
anchor test
```

### 🌐 Mainnet Testing

#### 1. Set the Solana Cluster to Mainnet

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

### 🧾 IDL Configuration

Ensure the generated IDL is named `peach_v_1.json`.

If it's your **first time deploying**, manually add the `metadata` field:

```json
"metadata": {
  "address": "your-program-id-here"
}
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