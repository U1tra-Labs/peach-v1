import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PeachV1 } from "../../target/types/peach_v1";
import { Keypair, PublicKey } from "@solana/web3.js";
import { createMint } from "@solana/spl-token";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";

export const provider = anchor.AnchorProvider.env();
anchor.setProvider(provider);
export const program = anchor.workspace.PeachV1 as Program<PeachV1>;
export const envProviderPayer = (provider.wallet as NodeWallet).payer;

// Constants
const MARKET_SEED = "Market";
const PEACH_ACCOUNT_SEED = "PeachAccount";
const BANK_SEED = "Bank";
const VAULT_SEED = "Vault";
const MINT_INFO_SEED = "MintInfo";

// Function to create a mint
export async function createTokenMint(decimals = 10): Promise<PublicKey> {
  return await createMint(
    program.provider.connection,
    envProviderPayer,
    provider.wallet.publicKey,
    provider.wallet.publicKey,
    decimals
  );
}

// Function to derive PDAs
export function derivePDAs(marketNum: number, accountNum: number, tokenIndex: number, mint: PublicKey) {
  const marketPDA = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from(MARKET_SEED), provider.wallet.publicKey.toBuffer(), new anchor.BN(marketNum).toArrayLike(Buffer, "le", 4)],
    program.programId
  )[0];

  const peachAccountPDA = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from(PEACH_ACCOUNT_SEED), marketPDA.toBuffer(), provider.wallet.publicKey.toBuffer(), new anchor.BN(accountNum).toArrayLike(Buffer, "le", 4)],
    program.programId
  )[0];

  const bankPDA = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from(BANK_SEED), marketPDA.toBuffer(), new anchor.BN(tokenIndex).toArrayLike(Buffer, "le", 2), new anchor.BN(0).toArrayLike(Buffer, "le", 4)],
    program.programId
  )[0];

  const vaultPDA = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from(VAULT_SEED), marketPDA.toBuffer(), new anchor.BN(tokenIndex).toArrayLike(Buffer, "le", 2), new anchor.BN(0).toArrayLike(Buffer, "le", 4)],
    program.programId
  )[0];

  const mintInfoPDA = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from(MINT_INFO_SEED), marketPDA.toBuffer(), mint.toBuffer()],
    program.programId
  )[0];

  return { marketPDA, peachAccountPDA, bankPDA, vaultPDA, mintInfoPDA };
}