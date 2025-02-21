use anchor_lang::zero_copy;
use anchor_lang::prelude::*;

#[account(zero_copy)]
pub struct PeachAccountFixed {
    pub market: Pubkey,
    pub owner: Pubkey,
    pub name: [u8; 32],
    pub delegate: Pubkey,
    pub account_num: u32,
    being_liquidated: u8,
    in_health_region: u8,
    pub bump: u8,
    pub sequence_number: u8,
    pub net_deposits: i64,
    pub health_region_begin_init_health: i64,
    pub frozen_until: u64,
    pub last_collateral_fee_charge: u64,
}

impl PeachAccountFixed {
    pub const SIZE: usize = 8+ 32 + 32 + 32 + 32 + 4 + 1 + 1 + 1 + 1 + 8 + 8 + 8 + 8;
}

impl PeachAccountFixed {
    pub fn name(&self) -> &str {
        std::str::from_utf8(&self.name)
            .unwrap()
            .trim_matches(char::from(0))
    }

    pub fn is_operational(&self) -> bool {
        let now_ts: u64 = Clock::get().unwrap().unix_timestamp.try_into().unwrap();
        self.frozen_until < now_ts
    }

    pub fn being_liquidated(&self) -> bool {
        self.being_liquidated == 1
    }

    pub fn set_being_liquidated(&mut self, b: bool) {
        self.being_liquidated = u8::from(b);
    }
}