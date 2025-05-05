use anchor_lang::prelude::*;
use solana_program::pubkey;

#[cfg(feature = "mainnet")]
pub const KAMINO_PROGRAM_ID: Pubkey = pubkey!("KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD");

#[cfg(not(feature = "mainnet"))]
pub const KAMINO_PROGRAM_ID: Pubkey = pubkey!("DrbgiNhrmpd3FhWCiQUXce3YkJQ6DcfUp49qmoCFYe2r"); // localnet/devnet fallback


