use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::state::Market;

pub fn market_create(
    ctx: Context<MarketCreate>,
    market_num: u32,
    testing: u8,
    version: u8,
) -> Result<()> {
    ctx.accounts.market.set_inner(
        Market { 
            creator: ctx.accounts.creator.key(), 
            market_num, 
            admin: ctx.accounts.creator.key(), 
            reserve_vault: ctx.accounts.reserve_vault.key(), 
            reserve_mint: ctx.accounts.reserve_mint.key(), 
            bump: ctx.bumps.market, 
            testing, 
            version, 
            deposit_limit_quote: 0, 
            ix_gate: 0, 
            collateral_fee_interval: 0
        }
    );

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
        space = 8 + 32 + 4 + 32 + 32 + 32 + 1 + 1 + 1+ 8 + 16 + 8,
    )]
    pub market: Account<'info, Market>,

    pub creator: Signer<'info>,

    pub reserve_mint: Account<'info, Mint>,

    #[account(
        init,
        seeds = [b"ReserveVault".as_ref(), market.key().as_ref()],
        bump,
        token::authority = market,
        token::mint = reserve_mint,
        payer = payer
    )]
    pub reserve_vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}
