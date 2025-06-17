import { PublicKey, Keypair } from "@solana/web3.js";
import { USDC_MINT_MAINNET, PYUSD_MINT_MAINNET, KAMINO_LENDING_MAIN_MARKET, KAMINO_LENDING } from "../helpers/const";
import { createTokenMint, deriveBankPDA, deriveVaultPDA, deriveMintInfoPDA, envProviderPayer, program } from "../helpers/setup";
import * as dotenv from "dotenv";
import * as fs from "fs";
import * as path from "path";
dotenv.config();

export enum TokenType {
  USDC = "USDC",
  PYUSD = "PYUSD",
  UNKNOWN = "UNKNOWN"
}

export function getTokenTypeFromName(name: string): TokenType {
  switch (name.trim().toUpperCase()) {
    case "USDC": return TokenType.USDC;
    case "PYUSD": return TokenType.PYUSD;
    default: return TokenType.UNKNOWN;
  }
}

/**
 * For devnet/localnet, check .env for a mint address for the token name.
 * If not found, create a new mint, write it to .env, and return it.
 * For mainnet, use the hardcoded mint.
 */
export async function getMintForTokenType(tokenType: TokenType, cluster: string, payer: Keypair): Promise<PublicKey> {
  if (cluster === "mainnet") {
    switch (tokenType) {
      case TokenType.USDC: return USDC_MINT_MAINNET;
      case TokenType.PYUSD: return PYUSD_MINT_MAINNET;
      default: throw new Error("Unknown token type for mainnet");
    }
  } else {
    // For devnet/localnet, check .env for a mint address
    const envKey = `${tokenType}_MINT_${cluster}`.toUpperCase();
    let mintStr = process.env[envKey];

    if (mintStr && mintStr.length > 0) {
      return new PublicKey(mintStr);
    }

    // Not found, create a new mint
    const mint = await createTokenMint(6, payer); // 6 decimals as example

    // Write to .env file
    const envPath = path.resolve(process.cwd(), ".env");
    let envContent = "";
    if (fs.existsSync(envPath)) {
      envContent = fs.readFileSync(envPath, "utf-8");
      // Remove any existing line for this key
      envContent = envContent.replace(new RegExp(`^${envKey}=.*$`, "m"), "");
      // Add a newline if not present at end
      if (!envContent.endsWith("\n")) envContent += "\n";
    }
    envContent += `${envKey}=${mint.toBase58()}\n`;
    fs.writeFileSync(envPath, envContent, "utf-8");

    // Also update process.env for this run
    process.env[envKey] = mint.toBase58();

    return mint;
  }
}

function getClusterFromEndpoint(endpoint: string): string {
  if (endpoint.includes("mainnet")) return "mainnet";
  if (endpoint.includes("devnet")) return "devnet";
  if (endpoint.includes("testnet")) return "testnet";
  return "localnet";
}

export class Token {
  mint: PublicKey;
  name?: string;
  bank: PublicKey[];
  vault: PublicKey[];
  mint_info: PublicKey;
  tokenIndex: number;
  programId: PublicKey;

  // Optional Kamino-related fields
  kaminoReserveMint?: PublicKey;
  kaminReserveFarmStateCollateral?: PublicKey;
  kaminReserveFarmStateDebt?: PublicKey;
  reserve?: PublicKey;
  reserve_liquidity_mint?: PublicKey;
  reserve_liquidity_supply?: PublicKey;
  reserve_collateral_mint?: PublicKey;
  reserve_collateral_supply?: PublicKey;
  liquidity_token_program?: PublicKey;
  collateral_token_program?: PublicKey;

  constructor(params: {
    mint: PublicKey;
    mint_info: PublicKey;
    bank?: PublicKey[];
    vault?: PublicKey[];
    name?: string;
    tokenIndex: number;
    programId: PublicKey;
    kaminoReserveMint?: PublicKey;
    kaminReserveFarmStateCollateral?: PublicKey;
    kaminReserveFarmStateDebt?: PublicKey;   
    reserve?: PublicKey;
    reserve_liquidity_mint?: PublicKey;
    reserve_liquidity_supply?: PublicKey;
    reserve_collateral_mint?: PublicKey;
    reserve_collateral_supply?: PublicKey;
    liquidity_token_program?: PublicKey;
    collateral_token_program?: PublicKey;
  }) {
    this.mint = params.mint;
    this.mint_info = params.mint_info;
    this.bank = params.bank ?? [];
    this.vault = params.vault ?? [];
    this.name = params.name;
    this.tokenIndex = params.tokenIndex;
    this.programId = params.programId;
    this.kaminoReserveMint = params.kaminoReserveMint;
    this.kaminReserveFarmStateCollateral = params.kaminReserveFarmStateCollateral; 
    this.kaminReserveFarmStateDebt = params.kaminReserveFarmStateDebt;             
    this.reserve = params.reserve;
    this.reserve_liquidity_mint = params.reserve_liquidity_mint;
    this.reserve_liquidity_supply = params.reserve_liquidity_supply;
    this.reserve_collateral_mint = params.reserve_collateral_mint;
    this.reserve_collateral_supply = params.reserve_collateral_supply;
    this.liquidity_token_program = params.liquidity_token_program;
    this.collateral_token_program = params.collateral_token_program;
  }

  display(): void {
    console.log('--- Token Info ---');
    if (this.name) console.log('Name:', this.name);
    console.log('Mint:', this.mint.toBase58());
    console.log('Mint Info:', this.mint_info.toBase58());
    console.log('Token Index:', this.tokenIndex);
    console.log('Program ID:', this.programId.toBase58());
    if (this.bank.length > 0) {
      console.log('Banks:', this.bank.map((b, i) => `  [${i}]: ${b.toBase58()}`).join(','));
    }
    if (this.vault.length > 0) {
      console.log('Vaults:', this.vault.map((v, i) => `  [${i}]: ${v.toBase58()}`).join(','));
    }
    if (this.kaminoReserveMint) {
      console.log('Kamino Reserve Mint:', this.kaminoReserveMint.toBase58());
    }
    if (this.kaminReserveFarmStateCollateral) {
      console.log('Kamin Reserve Farm State Collateral:', this.kaminReserveFarmStateCollateral.toBase58());
    }
    if (this.kaminReserveFarmStateDebt) {
      console.log('Kamin Reserve Farm State Debt:', this.kaminReserveFarmStateDebt.toBase58());
    }
    if (this.reserve) {
      console.log('Reserve:', this.reserve.toBase58());
    }
    if (this.reserve_liquidity_mint) {
      console.log('Reserve Liquidity Mint:', this.reserve_liquidity_mint.toBase58());
    }
    if (this.reserve_liquidity_supply) {
      console.log('Reserve Liquidity Supply:', this.reserve_liquidity_supply.toBase58());
    }
    if (this.reserve_collateral_mint) {
      console.log('Reserve Collateral Mint:', this.reserve_collateral_mint.toBase58());
    }
    if (this.reserve_collateral_supply) {
      console.log('Reserve Collateral Supply:', this.reserve_collateral_supply.toBase58());
    }
    if (this.liquidity_token_program) {
      console.log('Liquidity Token Program:', this.liquidity_token_program.toBase58());
    }
    if (this.collateral_token_program) {
      console.log('Collateral Token Program:', this.collateral_token_program.toBase58());
    }
  }

  /**
   * Extend this token with Kamino reserve mint and farm state.
   */
  extendToKamino(
    reserve: PublicKey,
    reserveFarmStateCollateral: PublicKey,
    reserveFarmStateDebt: PublicKey
  ): this {
    this.reserve = reserve;
    this.kaminReserveFarmStateCollateral = reserveFarmStateCollateral;
    this.kaminReserveFarmStateDebt = reserveFarmStateDebt;
    this.reserve_liquidity_mint = this.mint;

    [this.reserve_liquidity_supply] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("reserve_liq_supply"),
        KAMINO_LENDING_MAIN_MARKET.toBuffer(),
        this.mint.toBuffer(),
      ],
      KAMINO_LENDING
    );

    [this.reserve_collateral_mint] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("reserve_coll_mint"),
        KAMINO_LENDING_MAIN_MARKET.toBuffer(),
        this.mint.toBuffer(),
      ],
      KAMINO_LENDING
    );

    [this.reserve_collateral_supply] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("reserve_coll_supply"),
        KAMINO_LENDING_MAIN_MARKET.toBuffer(),
        this.mint.toBuffer(),
      ],
      KAMINO_LENDING
    );

    return this;
  }

  // Derive and set bank, vault, mint_info PDAs for a given market and tokenIndex
  deriveAndAppendPDAs(market: PublicKey, tokenIndex: number) {
    const bankPDA = deriveBankPDA(market, tokenIndex, 0);
    const vaultPDA = deriveVaultPDA(market, tokenIndex, 0);
    // Optionally, update mint_info if needed:
    // this.mint_info = deriveMintInfoPDA(market, this.mint);

    this.bank.push(bankPDA);
    this.vault.push(vaultPDA);
  }

  // Optionally, allow appending banks/vaults individually
  appendBank(bank: PublicKey) {
    this.bank.push(bank);
  }
  appendVault(vault: PublicKey) {
    this.vault.push(vault);
  }

  static builder() {
    return new TokenBuilder();
  }
}

export class TokenBuilder {
  private _name?: string;
  private _market?: PublicKey;
  private _tokenIndex?: number;
  private _programId?: PublicKey;
  private _kaminReserveFarmStateCollateral?: PublicKey;
  private _kaminReserveFarmStateDebt?: PublicKey;     

  /**
   * Set the token name (e.g. "USDC", "PYUSD")
   */
  name(name: string): this {
    this._name = name;
    return this;
  }

  /**
   * Set the market PDA for PDA derivation
   */
  market(market: PublicKey): this {
    this._market = market;
    return this;
  }

  /**
   * Set the token index for PDA derivation
   */
  tokenIndex(tokenIndex: number): this {
    this._tokenIndex = tokenIndex;
    return this;
  }

  /**
   * Set the programId for this token.
   */
  programId(programId: PublicKey): this {
    this._programId = programId;
    return this;
  }

  kaminReserveFarmStateCollateral(pubkey: PublicKey): this {
    this._kaminReserveFarmStateCollateral = pubkey;
    return this;
  }
  kaminReserveFarmStateDebt(pubkey: PublicKey): this {
    this._kaminReserveFarmStateDebt = pubkey;
    return this;
  }

  /**
   * Build the Token object.
   * - Uses process.env.TEST_ENV for environment.
   * - Uses program wallet as payer for devnet/localnet.
   * - Derives mint_info, bank, and vault.
   */
  async build(): Promise<Token> {
    if (!this._name) throw new Error("Token name is required");
    if (!this._market) throw new Error("Market PDA is required");
    if (this._tokenIndex === undefined) throw new Error("Token index is required");
    if (!this._programId) throw new Error("programId is required");

    // Determine cluster from program.provider.connection.rpcEndpoint
    const endpoint = program.provider.connection.rpcEndpoint;
    const cluster = getClusterFromEndpoint(endpoint);

    // Use program wallet as payer for devnet/localnet
    const payer = cluster === "mainnet"
      ? undefined
      : envProviderPayer;

    const tokenType = getTokenTypeFromName(this._name);

    // Get or create mint (now always async)
    const mint = await getMintForTokenType(tokenType, cluster, payer!);

    // Derive mint_info
    const mint_info = deriveMintInfoPDA(this._market, mint);

    // Derive bank and vault
    const bank = [deriveBankPDA(this._market, this._tokenIndex, 0)];
    const vault = [deriveVaultPDA(this._market, this._tokenIndex, 0)];

    return new Token({
      mint,
      mint_info,
      bank,
      vault,
      name: this._name,
      tokenIndex: this._tokenIndex,
      programId: this._programId,
      kaminReserveFarmStateCollateral: this._kaminReserveFarmStateCollateral,
      kaminReserveFarmStateDebt: this._kaminReserveFarmStateDebt,           
    });
  }
}