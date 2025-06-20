use anchor_lang::prelude::*;

use crate::state::Market;

pub fn market_create(
    ctx: Context<MarketCreate>,
    market_num: u32,
    testing: u8,
    version: u8,
) -> Result<()> {
    let mut market = ctx.accounts.market.load_init()?;
    market.creator = ctx.accounts.creator.key();
    market.market_num = market_num;
    market.admin = ctx.accounts.creator.key();
    market.bump = ctx.bumps.market;
    market.testing = testing;
    market.version = version;

    Ok(())
}

#[derive(Accounts)]
#[instruction(market_num: u32)]
pub struct MarketCreate<'info> {
    #[account(
        init,
        seeds = [b"Market".as_ref(), creator.key().as_ref(), &market_num.to_le_bytes()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<Market>(),
    )]
    pub market: AccountLoader<'info, Market>,

    pub creator: Signer<'info>,
    
    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}
