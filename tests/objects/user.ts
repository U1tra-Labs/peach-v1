import { Keypair, PublicKey } from "@solana/web3.js";
import { derivePeachAccountPDA } from "../helpers/setup";
import { getFundedWallet } from "../helpers/fundWallets";
import { KAMINO_LENDING, KAMINO_FARM_MAINNET, KAMINO_LENDING_MAIN_MARKET, SEED1_Account, SEED2_Account } from "../helpers/const";
import { Token } from "./token";

// Instruction: InitObligation
export interface InitObligationArgs {
  tag: number;
  id: number;
}

export interface ObligationFarms {
  colalteralFarm: PublicKey;
  debtFarm: PublicKey;
}

export class User {
  wallet: Keypair;
  peachAccount: PublicKey;
  tokenAccounts: PublicKey[];
  accountNum: number;
  obligation?: PublicKey;
  userMetadata?: PublicKey;
  obligationFarms: ObligationFarms[];

  constructor(params: {
    wallet: Keypair;
    peachAccount: PublicKey;
    accountNum: number;
    tokenAccounts?: PublicKey[];
    obligation?: PublicKey;
    userMetadata?: PublicKey;
    obligationFarms?: ObligationFarms[];
  }) {
    this.wallet = params.wallet;
    this.peachAccount = params.peachAccount;
    this.tokenAccounts = params.tokenAccounts ?? [];
    this.accountNum = params.accountNum;
    this.obligation = params.obligation;
    this.userMetadata = params.userMetadata;
    this.obligationFarms = params.obligationFarms ?? [];
  }

  addTokenAccount(tokenAccount: PublicKey) {
    this.tokenAccounts.push(tokenAccount);
  }

  addTokenAccounts(tokenAccounts: PublicKey[]) {
    this.tokenAccounts.push(...tokenAccounts);
  }

  setObligation(obligation: PublicKey) {
    this.obligation = obligation;
  }

  setUserMetadata(userMetadata: PublicKey) {
    this.userMetadata = userMetadata;
  }

  addObligationFarm(obligationFarm: ObligationFarms) {
    if (!this.obligationFarms) this.obligationFarms = [];
    this.obligationFarms.push(obligationFarm);
  }

  addObligationFarms(obligationFarms: ObligationFarms[]) {
    if (!this.obligationFarms) this.obligationFarms = [];
    this.obligationFarms.push(...obligationFarms);
  }

  getObligationFarmForToken(tokenIndex: number): ObligationFarms | undefined {
    return this.obligationFarms[tokenIndex-1];
  }

  getTokenAccount(tokenIndex: number): PublicKey | undefined {
    return this.tokenAccounts[tokenIndex-1];
  }

  /**
   * Derives and sets Kamino obligation, userMetadata, and obligationFarms.
   * @param args - args for obligation PDA derivation
   * @param tokens - array of Token objects (must have kaminReserveFarmState)
   */
  async extendToKaminoUser(
    args: InitObligationArgs,
    tokens: Token[]
  ): Promise<this> {
    [this.userMetadata] = PublicKey.findProgramAddressSync(
      [Buffer.from("user_meta"), this.wallet.publicKey.toBuffer()],
      KAMINO_LENDING
    );

    const lendingMarket = KAMINO_LENDING_MAIN_MARKET;
    const seed1Account = SEED1_Account;
    const seed2Account = SEED2_Account;

    [this.obligation] = PublicKey.findProgramAddressSync(
      [
        Buffer.from(Uint8Array.of(args.tag)),
        Buffer.from(Uint8Array.of(args.id)),
        this.wallet.publicKey.toBuffer(),
        lendingMarket.toBuffer(),
        seed1Account.toBuffer(),
        seed2Account.toBuffer(),
      ],
      KAMINO_LENDING
    );

    const obligationFarms = [];
    for (const token of tokens) {
      if (!token.reserve || !token.kaminReserveFarmStateCollateral || !token.kaminReserveFarmStateDebt) {
        throw new Error("Token must have reserve and kaminReserveFarmState set");
      }
      const [obligationFarmCollateral] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("user"),
          token.kaminReserveFarmStateCollateral.toBuffer(),
          this.obligation.toBuffer(),
        ],
        KAMINO_FARM_MAINNET
      );

      const [obligationFarmDebt] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("user"),
          token.kaminReserveFarmStateDebt.toBuffer(),
          this.obligation.toBuffer(),
        ],
        KAMINO_FARM_MAINNET
      );

      const obligationFarm: ObligationFarms = {
        colalteralFarm: obligationFarmCollateral,
        debtFarm: obligationFarmDebt,
      };

      obligationFarms.push(obligationFarm);
    }

    this.addObligationFarms(obligationFarms);

    return this;
  }

  display(): void {
    console.log("--- User Info ---");
    console.log("Wallet pubkey:", this.wallet.publicKey.toBase58());
    console.log("Peach Account:", this.peachAccount.toBase58());
    console.log("Account Num:", this.accountNum);
    if (this.obligation) console.log("Obligation:", this.obligation.toBase58());
    if (this.userMetadata) console.log("User Metadata:", this.userMetadata.toBase58());
    console.log("Token Accounts:");
    this.tokenAccounts.forEach((ta, i) =>
      console.log(`  [${i}]: ${ta.toBase58()}`)
    );
    if (this.obligationFarms.length > 0) {
      console.log("Obligation Farms:");
      this.obligationFarms.forEach((of, i) => {
        console.log(`  [${i}] Collateral Farm: ${of.colalteralFarm.toBase58()}`);
        console.log(`  [${i}] Debt Farm:       ${of.debtFarm.toBase58()}`);
      });
    }
  }

  static builder() {
    return new UserBuilder();
  }
}

export class UserBuilder {
  private _wallet?: Keypair;
  private _market?: PublicKey;
  private _accountNum?: number;
  private _tokenAccounts: PublicKey[] = [];

  async walletFromFaucet(index?: number): Promise<this> {
    this._wallet = await getFundedWallet(index);
    return this;
  }

  wallet(wallet: Keypair): this {
    this._wallet = wallet;
    return this;
  }

  market(market: PublicKey): this {
    this._market = market;
    return this;
  }

  accountNum(accountNum: number): this {
    this._accountNum = accountNum;
    return this;
  }

  tokenAccounts(tokenAccounts: PublicKey[]): this {
    this._tokenAccounts = tokenAccounts;
    return this;
  }

  /**
   * Build the User object.
   * Kamino obligation, userMetadata, kaminoTokenAccounts, and obligationFarms are not set here; use extendToKaminoUser() after building.
   */
  async build(): Promise<User> {
    if (!this._market) throw new Error("Market PDA is required");
    if (this._accountNum === undefined) throw new Error("Account number is required");

    // Fetch or get funded wallet  
    const wallet = await getFundedWallet(this._accountNum).catch(() => {
      throw new Error(`Failed to get funded wallet for account ${this._accountNum}`);
    });

    // Derive peach account PDA
    const peachAccount = derivePeachAccountPDA(this._market, this._accountNum, wallet.publicKey);

    return new User({
      wallet,
      peachAccount,
      accountNum: this._accountNum,
      tokenAccounts: this._tokenAccounts,
    });
  }
}