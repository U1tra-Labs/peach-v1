/**
 * Helpers that make Peach user transactions consume REAL Pyth prices on
 * Solana mainnet-beta.
 *
 * Pattern A (recommended, works with the current program unchanged):
 *   - Admin registers each token with `oracle = priceFeedPDA(feedId, shard)` and
 *     `max_staleness_slots ≈ 60` (see `registerTokenWithPyth` below).
 *   - The keeper (`keeper.ts`) calls `update_price_feed` every ~10s to keep those
 *     PDAs fresh.
 *   - Deposits / withdraws pass the SAME PDA as `oracle`. On-chain
 *     `determine_oracle_type` sees owner == Pyth receiver => `OracleType::PythV2`,
 *     `get_pyth_on_demand_state` requires `VerificationLevel::Full`, and the
 *     staleness/confidence gates in `Bank::oracle_price` enforce liveness.
 *
 * Pattern B (freshest possible, same-tx bundle):
 *   - `depositWithLivePrices` / `withdrawWithLivePrices` fetch Hermes in the client,
 *     then build ONE Pyth transaction-builder sequence:
 *       [update_price_feed(PDA) ..., peach_token_deposit(oracle=PDA) ]
 *     The price-feed PDA is updated and consumed in the same bundle, so the Peach
 *     instruction always sees a price posted seconds ago even if the background
 *     keeper is delayed. Use this for user-facing deposit/withdraw flows on mainnet.
 *     The Pyth client wallet pays for the update txs; pass the user's Keypair as
 *     `signer` (it can be the same wallet) so the Peach instruction is authorized.
 */
import {
  Connection,
  PublicKey,
  TransactionInstruction,
  Keypair,
} from "@solana/web3.js";
import { Program, Wallet } from "@coral-xyz/anchor";
import { HermesClient } from "@pythnetwork/hermes-client";
import {
  PythSolanaReceiver,
  InstructionWithEphemeralSigners,
} from "@pythnetwork/pyth-solana-receiver";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import * as anchor from "@coral-xyz/anchor";

export interface PythClient {
  connection: Connection;
  wallet: Wallet;
  receiver: PythSolanaReceiver;
  hermes: HermesClient;
  shardId: number;
}

export function makePythClient(opts: {
  connection: Connection;
  wallet: Wallet;
  hermesUrl?: string;
  pythApiKey?: string;
  shardId?: number;
}): PythClient {
  const hermesUrl =
    opts.hermesUrl ?? process.env.HERMES_URL ?? "https://hermes.pyth.network";
  const pythApiKey = opts.pythApiKey ?? process.env.PYTH_API_KEY;
  // PythSolanaReceiver builds its own AnchorProvider internally.
  const receiver = new PythSolanaReceiver({
    connection: opts.connection,
    wallet: opts.wallet,
  });
  const hermes = new HermesClient(
    hermesUrl,
    pythApiKey ? { accessToken: pythApiKey } : undefined
  );
  return {
    connection: opts.connection,
    wallet: opts.wallet,
    receiver,
    hermes,
    shardId: opts.shardId ?? Number(process.env.PYTH_SHARD_ID ?? "0"),
  };
}

/** Fixed price-feed PDA for a feed id on our shard — the value to store in Bank.oracle. */
export function priceFeedPda(client: PythClient, feedId: string): PublicKey {
  return client.receiver.getPriceFeedAccountAddress(client.shardId, feedId);
}

/** Fetch Hermes signed updates for a set of feed ids (base64, ready for the receiver). */
export async function fetchPriceUpdateData(
  client: PythClient,
  feedIds: string[]
): Promise<string[]> {
  return (
    await client.hermes.getLatestPriceUpdates(feedIds, { encoding: "base64" })
  ).binary.data as string[];
}

/**
 * Post price-feed updates AND run Peach consumer instructions in one atomic
 * builder sequence. `buildConsumerIx` receives `getPriceFeedAccount(feedId)` which
 * resolves to the fixed PDA for our shard — pass it as `oracle` (and health
 * remaining-accounts) in the Peach ix.
 */
export async function postPricesAndConsume(
  client: PythClient,
  feedIds: string[],
  buildConsumerIx: (
    getPriceFeedAccount: (feedId: string) => PublicKey
  ) => Promise<InstructionWithEphemeralSigners[]>,
  opts?: { computeUnitPriceMicroLamports?: number }
): Promise<string[]> {
  const priceUpdateData = await fetchPriceUpdateData(client, feedIds);
  const builder = client.receiver.newTransactionBuilder({});
  await builder.addUpdatePriceFeed(priceUpdateData, client.shardId);
  await builder.addPriceConsumerInstructions(buildConsumerIx);
  const txs = await builder.buildVersionedTransactions({
    computeUnitPriceMicroLamports: opts?.computeUnitPriceMicroLamports ?? 50000,
  });
  return await client.receiver.provider.sendAll(txs, {
    skipPreflight: true,
  } as any);
}

/** Convenience: token_register pointed at a live Pyth price-feed PDA. */
export async function registerTokenWithPyth(
  peach: Program,
  args: {
    tokenIndex: number;
    name: string;
    market: PublicKey;
    mint: PublicKey;
    bank: PublicKey;
    vault: PublicKey;
    mintInfo: PublicKey;
    priceFeedAccount: PublicKey;
    admin: Keypair;
    payer?: PublicKey;
  }
): Promise<string> {
  const payer = args.payer ?? args.admin.publicKey;
  const vaultIx = await (peach.methods as any)
    // TokenIndex is a transparent newtype: the coder expects { 0: n }.
    .tokenVaultCreate({ 0: args.tokenIndex })
    .accounts({
      market: args.market,
      admin: args.admin.publicKey,
      mint: args.mint,
      vault: args.vault,
      payer,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: anchor.web3.SystemProgram.programId,
      rent: anchor.web3.SYSVAR_RENT_PUBKEY,
    })
    .instruction();

  // Mirrors tests/instructions/token.ts defaults, but with the Pyth PDA as
  // both oracle and fallback, and a mainnet-sensible staleness window.
  const registerIx = await (peach.methods as any)
    .tokenRegister(
      { 0: args.tokenIndex },
      args.name,
      { confFilter: 0.1, maxStalenessSlots: 60 },
      {
        util0: 0.8,
        rate0: 0.02,
        util1: 0.9,
        rate1: 0.05,
        maxRate: 0.1,
        adjustmentFactor: 0.01,
      },
      0.002,
      0.001,
      0.8,
      0.7,
      1.2,
      1.3,
      0.05,
      60,
      0.02,
      0.03,
      0.1,
      new anchor.BN(600),
      new anchor.BN(1000000),
      50000,
      50000,
      0,
      1,
      0.8,
      true,
      new anchor.BN(1000000),
      0.01,
      0.02,
      false,
      0.001,
      "tier"
    )
    .accounts({
      market: args.market,
      admin: args.admin.publicKey,
      mint: args.mint,
      bank: args.bank,
      vault: args.vault,
      mintInfo: args.mintInfo,
      oracle: args.priceFeedAccount,
      fallbackOracle: args.priceFeedAccount,
      payer,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: anchor.web3.SystemProgram.programId,
      rent: anchor.web3.SYSVAR_RENT_PUBKEY,
    })
    .instruction();

  const tx = new anchor.web3.Transaction().add(vaultIx).add(registerIx);
  return await peach.provider.sendAndConfirm(tx, [args.admin]);
}

/** Build a Peach token_deposit ix that reads the given Pyth price-feed PDA. */
export async function buildDepositIx(
  peach: Program,
  args: {
    amount: anchor.BN;
    market: PublicKey;
    account: PublicKey;
    owner: PublicKey;
    bank: PublicKey;
    vault: PublicKey;
    priceFeedAccount: PublicKey;
    userTokenAccount: PublicKey;
    tokenAuthority: PublicKey;
  }
): Promise<TransactionInstruction> {
  return await (peach.methods as any)
    .tokenDeposit(args.amount, false)
    .accounts({
      market: args.market,
      account: args.account,
      owner: args.owner,
      bank: args.bank,
      vault: args.vault,
      oracle: args.priceFeedAccount,
      tokenAccount: args.userTokenAccount,
      tokenAuthority: args.tokenAuthority,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .instruction();
}

/** Deposit with a guaranteed-fresh Pyth price: update PDA + deposit in one bundle. */
export async function depositWithLivePrices(
  peach: Program,
  pyth: PythClient,
  feedId: string,
  deposit: Parameters<typeof buildDepositIx>[1] & { signer: Keypair }
): Promise<string[]> {
  return await postPricesAndConsume(pyth, [feedId], async (getAccount) => [
    {
      instruction: await buildDepositIx(peach, {
        ...deposit,
        priceFeedAccount: getAccount(feedId),
      }),
      signers: [deposit.signer],
    },
  ]);
}

/** Build a Peach token_withdraw ix that reads the given Pyth price-feed PDA. */
export async function buildWithdrawIx(
  peach: Program,
  args: {
    amount: anchor.BN;
    allowBorrow: boolean;
    market: PublicKey;
    account: PublicKey;
    owner: PublicKey;
    bank: PublicKey;
    vault: PublicKey;
    priceFeedAccount: PublicKey;
    userTokenAccount: PublicKey;
  }
): Promise<TransactionInstruction> {
  return await (peach.methods as any)
    .tokenWithdraw(args.amount, args.allowBorrow)
    .accounts({
      market: args.market,
      account: args.account,
      owner: args.owner,
      bank: args.bank,
      vault: args.vault,
      oracle: args.priceFeedAccount,
      tokenAccount: args.userTokenAccount,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .instruction();
}

/** Withdraw/borrow with a guaranteed-fresh Pyth price in one bundle. */
export async function withdrawWithLivePrices(
  peach: Program,
  pyth: PythClient,
  feedId: string,
  withdraw: Parameters<typeof buildWithdrawIx>[1] & { signer: Keypair }
): Promise<string[]> {
  return await postPricesAndConsume(pyth, [feedId], async (getAccount) => [
    {
      instruction: await buildWithdrawIx(peach, {
        ...withdraw,
        priceFeedAccount: getAccount(feedId),
      }),
      signers: [withdraw.signer],
    },
  ]);
}
