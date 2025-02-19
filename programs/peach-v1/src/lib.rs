use anchor_lang::prelude::*;
use instructions::*;

pub mod instructions;
pub mod state;

declare_id!("EKrZVNcqcS4uJLAq9DuAPP5ewX45XnfaYVy9367uzK2K");

#[program]
pub mod peach_v1 {

    use super::*;

    pub fn market_create(
        ctx: Context<MarketCreate>, 
        market_num: u32,
        testing: u8,
        version: u8
    ) -> Result<()> {
        instructions::market_create(ctx, market_num, testing, version)
    }
}

#[derive(Accounts)]
pub struct Initialize {}
