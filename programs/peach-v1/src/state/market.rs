use anchor_lang::prelude::*;

#[account(zero_copy)]
#[derive(Debug)]
pub struct Market {
    pub creator: Pubkey,
    pub market_num: u32,
    pub admin: Pubkey,
    pub bump: u8,
    pub testing: u8,
    pub version: u8,
    pub padding: [u8; 1],
    pub deposit_limit_quote: u64,
    pub ix_gate: u128,
    pub collateral_fee_interval: u64
}

impl Market {
    pub const SIZE: usize = 8 + 32 + 4 + 32 + 32 + 32 + 1 + 1 + 1+ 1+ 8 + 16 + 8;

    pub fn is_ix_enabled(&self, ix: IxGate) -> bool {
        self.ix_gate & (1 << ix as u128) == 0
    }
}

#[derive(Copy, Clone, Debug)]
pub enum IxGate {
    AccountCreate = 0,
    GroupCreate = 1,
}
