import { PublicKey, SystemProgram } from "@solana/web3.js";

export const LAMPORTS_PER_SOL_FOR_TEST_WALLETS = 0.1; // 0.1 SOL for each wallet

export const PYTH_USDC_ORACLE = new PublicKey('Dpw1EAVrSB1ibxiDQyTAW6Zip3J4Btk2x4SgApQCeFbX');

// Kamino

export const SEED1_Account = SystemProgram.programId;
export const SEED2_Account = SystemProgram.programId;

export const KAMINO_LENDING = new PublicKey(
    "KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD" // Kamino mainnet
    // "DrbgiNhrmpd3FhWCiQUXce3YkJQ6DcfUp49qmoCFYe2r" // Kamino localhost
);

export const KAMINO_FARM_MAINNET = new PublicKey(
    "FarmsPZpWu9i7Kky8tPN37rs2TpmMrAZrC7S7vJa91Hr");

export const KAMINO_LENDING_MAIN_MARKET = new PublicKey(
    "7u3HeHxYDLhnCoErrtycNokbQYbWGzLs6JSDqGAv5PfF"
);

export const KAMINO_LENDING_MAIN_MARKET_AUTHORITY = new PublicKey(
    "9DrvZvyWh1HuAoZxvYWMvkf2XCzryCpGgHqrMjyDWpmo" // This is the same as KAMINO_LENDING_MAIN_MARKET
);

export const KAMINO_SCOPE_PRICES = new PublicKey(
    "3NJYftD5sjVfxSnUdZ1wVML8f3aC6mp1CXCL6L7TnU8C");


// USDC

export const USDC_MINT_MAINNET = new PublicKey(
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
);

export const KAMINO_RESERVE_STATE_USDC =  new PublicKey(
    "D6q6wuQSrifJKZYpR1M8R4YawnLDtDsMmWM1NbBmgJ59"
);

export const KAMINO_RESERVE_FARM_STATE_USDC_COLLATERAL = new PublicKey(
    "JAvnB9AKtgPsTEoKmn24Bq64UMoYcrtWtq42HHBdsPkh");

// TODO: Add the correct USDC debt farm state public key
export const KAMINO_RESERVE_FARM_STATE_USDC_DEBT = new PublicKey(
"JAvnB9AKtgPsTEoKmn24Bq64UMoYcrtWtq42HHBdsPkh");


// PYUSD

export const PYUSD_MINT_MAINNET = new PublicKey(
    "2b1kV6DkPAnxd5ixfnxCpjxmKwqjjaYmCZfHsFu24GXo");

export const KAMINO_RESERVE_STATE_PYUSD = new PublicKey(
    "2gc9Dm1eB6UgVYFBUN9bWks6Kes9PbWSaPaa9DqyvEiN");

export const KAMINO_RESERVE_FARM_STATE_PYUSD_COLLATERAL = new PublicKey(
    "DEe2NZ5dAXGxC7M8Gs9Esd9wZRPdQzG8jNamXqhL5yku"
);

export const KAMINO_RESERVE_FARM_STATE_PYUSD_DEBT = new PublicKey(
    "GmJ2vXsDt8R5DNimAZc7Rtphr4oqecBVAx1psaTcVtrX");

export const KAMINO_RESERVE_LIQUIDITY_FEE_VAULT_PYUSD = new PublicKey(
    "BcLJRx7GbyX2Jj8RFpYDnEE47Tm36wSskLnm7ALarEC1");

// export const KAMINO_RESERVE_LIQUIDITY_SUPPLY = new PublicKey(
//     "Gm2itCNPBpBSSrgCA194pmErjwHAFVpvBBFvpdTF5LuJ");

// export const KAMINO_RESERVE_USDC_COLLATERAL_MINT = new PublicKey(
//     "B8V6WVjPxW1UGwVDfxH2d2r8SyT4cqn7dQRK6XneVa7D");