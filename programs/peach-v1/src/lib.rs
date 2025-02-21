use anchor_lang::prelude::*;
use instructions::*;

#[macro_use]
pub mod util;

pub mod instructions;
pub mod state;
pub mod error;

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

        pub fn account_create(
        ctx: Context<AccountCreate>,
        account_num: u32,
        token_count: u8,
        name: String,
    ) -> Result<()> {
        instructions::account_create(
            ctx,
            account_num,
            token_count,
            name,
        )
    }
}
