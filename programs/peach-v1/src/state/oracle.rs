use anchor_lang::prelude::*;
use anchor_lang::AnchorDeserialize;
use static_assertions::const_assert_eq;
use std::mem::size_of;


use fixed::types::I80F48;
use derivative::Derivative;

#[zero_copy]
#[derive(AnchorDeserialize, AnchorSerialize, Derivative, PartialEq, Eq)]
#[derivative(Debug)]
pub struct OracleConfig {
    pub conf_filter: I80F48,
    pub max_staleness_slots: i64,
    #[derivative(Debug = "ignore")]
    pub reserved: [u8; 72],
}

#[derive(AnchorDeserialize, AnchorSerialize, Debug, Default)]
pub struct OracleConfigParams {
    pub conf_filter: f32,
    pub max_staleness_slots: Option<u32>,
}

impl OracleConfigParams {
    pub fn to_oracle_config(&self) -> OracleConfig {
        OracleConfig {
            conf_filter: I80F48::from_num(self.conf_filter),
            max_staleness_slots: self.max_staleness_slots.map(|v| v as i64).unwrap_or(-1),
            reserved: [0; 72],
        }
    }
}

#[account(zero_copy)]
pub struct StubOracle {
    // ABI: Clients rely on this being at offset 8
    pub group: Pubkey,
    // ABI: Clients rely on this being at offset 40
    pub mint: Pubkey,
    pub price: I80F48,
    pub last_update_ts: i64,
    pub last_update_slot: u64,
    pub deviation: I80F48,
    pub reserved: [u8; 104],
}
const_assert_eq!(size_of::<StubOracle>(), 32 + 32 + 16 + 8 + 8 + 16 + 104);
const_assert_eq!(size_of::<StubOracle>(), 216);
const_assert_eq!(size_of::<StubOracle>() % 8, 0);