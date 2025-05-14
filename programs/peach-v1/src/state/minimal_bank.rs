// use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};
use anchor_lang::prelude::Pubkey; // For Pubkey

// Assuming these paths are correct based on your project structure
use crate::state::oracle::OracleConfig;
use crate::state::stable_price::StablePriceModel;
use crate::state::bank::MyFixedIdlWrapper;

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy, Default, Debug)] // Added Default and Debug for easier testing
pub struct MinimalBank {
    pub oracle_config: OracleConfig,
    pub stable_price_model: StablePriceModel,
    pub wrapper: MyFixedIdlWrapper,
    // Added fields:
    pub deposit_index: MyFixedIdlWrapper,
    pub borrow_index: MyFixedIdlWrapper,
    pub index_last_updated: u64,
    pub bank_rate_last_updated: u64,
    // Newly added fields
    pub market: Pubkey,                // Example Pubkey field
    pub name: [u8; 16],                // Example [u8; 16] array
}

// Implementing Default for nested types if they don't have it or to be explicit.
// MyFixedIdlWrapper already derives Default.
// OracleConfig has a Default impl.
// StablePriceModel has a Default impl.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_bank_pod_check() {
        // Should successfully transmute without size errors if MinimalBank is Pod
        let bank_instance = MinimalBank {
            oracle_config: Default::default(),
            stable_price_model: Default::default(),
            wrapper: Default::default(),
            // Initialize new fields
            deposit_index: Default::default(),
            borrow_index: Default::default(),
            index_last_updated: 0,
            bank_rate_last_updated: 0,
            market: Pubkey::default(),
            name: [0u8; 16],
        };
        let _bytes: &[u8] = bytemuck::bytes_of(&bank_instance);
        // const EXPECTED_SIZE: usize = core::mem::size_of::<OracleConfig>() +
        //                              core::mem::size_of::<StablePriceModel>() +
        //                              core::mem::size_of::<MyFixedIdlWrapper>() * 3 +
        //                              core::mem::size_of::<u64>() * 2 +
        //                              core::mem::size_of::<Pubkey>() +
        //                              16; // size of [u8; 16]
        // assert_eq!(core::mem::size_of_val(&bank_instance), EXPECTED_SIZE, "Size mismatch for MinimalBank");
    }
} 