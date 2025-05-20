import { PublicKey } from "@solana/web3.js";

export const LAMPORTS_PER_SOL_FOR_TEST_WALLETS = 0.1; // 0.1 SOL for each wallet

export const TEST_ENV = "localhost"; // "devnet" | "localhost" | "mainnet"

export const PYTH_USDC_ORACLE = new PublicKey('Dpw1EAVrSB1ibxiDQyTAW6Zip3J4Btk2x4SgApQCeFbX');

// Kamino 

export const KAMINO_LENDING = new PublicKey(
    "KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD" // Kamino mainnet
    // "DrbgiNhrmpd3FhWCiQUXce3YkJQ6DcfUp49qmoCFYe2r" // Kamino localhost
);

export const KAMINO_LENDING_MAIN_MARKET = new PublicKey(
    "7u3HeHxYDLhnCoErrtycNokbQYbWGzLs6JSDqGAv5PfF"
);

export const KAMINO_RESERVE_FARM_STATE_USDC = new PublicKey(
    "JAvnB9AKtgPsTEoKmn24Bq64UMoYcrtWtq42HHBdsPkh");

export const KAMINO_FARM_MAINNET = new PublicKey(
    "FarmsPZpWu9i7Kky8tPN37rs2TpmMrAZrC7S7vJa91Hr");

export const KAMINO_RESERVE_USDC =  new PublicKey(
        "D6q6wuQSrifJKZYpR1M8R4YawnLDtDsMmWM1NbBmgJ59"
);
    
export const USDC_MINT_MAINNET = new PublicKey(
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
);

export const KAMINO_RESERVED_USDC_MINT = new PublicKey(
    "B8V6WVjPxW1UGwVDfxH2d2r8SyT4cqn7dQRK6XneVa7D");