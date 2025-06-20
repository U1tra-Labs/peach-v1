use anchor_lang::prelude::*;
use derivative::Derivative;

use crate::custom_types::TokenIndex;
use crate::error::*;

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

    pub market_insurance_fund: u8,
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

impl MintInfo {
    pub fn num_banks(&self) -> usize {
        self.banks
            .iter()
            .position(|&b| b == Pubkey::default())
            .unwrap_or(MAX_BANKS)
    }

    pub fn banks(&self) -> &[Pubkey] {
        &self.banks[..self.num_banks()]
    }

    pub fn verify_banks_ais(&self, all_bank_ais: &[AccountInfo]) -> Result<()> {
        require_msg!(
            all_bank_ais.iter().map(|ai| ai.key).eq(self.banks().iter()),
            "the passed banks {:?} don't match banks in mint_info {:?}",
            all_bank_ais.iter().map(|ai| ai.key).collect::<Vec<_>>(),
            self.banks()
        );
        Ok(())
    }
}