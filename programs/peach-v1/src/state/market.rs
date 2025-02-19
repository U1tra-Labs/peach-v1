use anchor_lang::prelude::*;

#[account]
#[derive(Debug)]
pub struct Market {
    pub creator: Pubkey,
    pub market_num: u32,
    pub admin: Pubkey,
    pub reserve_vault: Pubkey,
    pub reserve_mint: Pubkey,
    pub bump: u8,
    pub testing: u8,
    pub version: u8,
    pub deposit_limit_quote: u64,
    pub ix_gate: u128,
    pub collateral_fee_interval: u64,
}