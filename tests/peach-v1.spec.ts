import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";
import { Keypair, PublicKey } from "@solana/web3.js";
import { provider, createTokenMint, derivePDAs } from "./helpers/setup";
import { createMarket, createPeachAccount, createStubOracle } from "./helpers/transactions";

// Test suite
describe("peach-v1", () => {
  const program = anchor.workspace.PeachV1;
  const stubOracle = Keypair.generate();
  let marketPDA: PublicKey, peachAccountPDA: PublicKey, bankPDA: PublicKey, vaultPDA: PublicKey, mintInfoPDA: PublicKey;
  let mint: PublicKey;

  // Update these values as needed
  let marketNum = 6, accountNum = 0, tokenIndex = 5, price = 1.0;

  before(async () => {
    // Create mint
    mint = await createTokenMint(10);

    // Derive PDAs
    ({ marketPDA, peachAccountPDA, bankPDA, vaultPDA, mintInfoPDA } = derivePDAs(marketNum, accountNum, tokenIndex, mint));

    console.log("Market PDA:", marketPDA.toBase58());
    console.log("Peach Account PDA:", peachAccountPDA.toBase58());
    console.log("Bank PDA:", bankPDA.toBase58());
    console.log("Vault PDA:", vaultPDA.toBase58());
    console.log("Mint Info PDA:", mintInfoPDA.toBase58());
  });

  it("Creates a market", async () => {
    const tx = await createMarket(marketPDA, marketNum);
    console.log("Market Created: ", tx);

    const market = await program.account.market.fetch(marketPDA);
    assert.equal(market.marketNum, marketNum);
  });

  it("Creates a peach account", async () => {
    const tx = await createPeachAccount(peachAccountPDA, marketPDA, accountNum);
    console.log("Peach Account Created: ", tx);

    const account = await program.account.peachAccountFixed.fetch(peachAccountPDA);
    assert.equal(account.accountNum, accountNum);
  });

  it("Creates a stub oracle", async () => {
    const tx = await createStubOracle(stubOracle, marketPDA, mint, price);
    console.log("Stub Oracle Created: ", tx);

    const oracle = await program.account.stubOracle.fetch(stubOracle.publicKey);
    assert.ok(oracle.mint.equals(mint));
  });
});

