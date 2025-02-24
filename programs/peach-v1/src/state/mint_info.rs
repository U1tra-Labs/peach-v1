use anchor_lang::prelude::*;
use derivative::Derivative;

use super::TokenIndex;

pub const MAX_BANKS: usize = 6;

// This struct describes which address lookup table can be used to pass
// the accounts that are relevant for this mint. The idea is that clients
// can load this account to figure out which address maps to use when calling
// instructions that need banks/oracles for all active positions.
#[account(zero_copy)]
#[derive(Derivative)]
#[derivative(Debug)]
pub struct MintInfo {
    // ABI: Clients rely on this being at offset 8
    pub market: Pubkey,

    // ABI: Clients rely on this being at offset 40
    pub token_index: TokenIndex,

    pub group_insurance_fund: u8,
    #[derivative(Debug = "ignore")]
    pub padding1: [u8; 5],
    pub mint: Pubkey,
    pub banks: [Pubkey; MAX_BANKS],
    pub vaults: [Pubkey; MAX_BANKS],
    pub oracle: Pubkey,

    pub registration_time: u64,

    pub fallback_oracle: Pubkey,

    // #[derivative(Debug = "ignore")]
    // pub reserved: [u8; 2528],
}