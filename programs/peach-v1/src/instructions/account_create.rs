use anchor_lang::prelude::*;

use crate::error::PeachError;
use crate::state::{IxGate, Market, PeachAccountFixed};

use crate::util::fill_from_str;

pub fn account_create(
    ctx: Context<AccountCreate>,
    account_num: u32,
    _token_count: u8,
    name: String,
) -> Result<()> {
    let mut account = ctx.accounts.account.load_init()?;

    account.name = fill_from_str(&name)?;
    account.market = ctx.accounts.market.key();
    account.owner = ctx.accounts.owner.key();
    account.account_num = account_num;
    account.bump = ctx.bumps.account;
    account.delegate = Pubkey::default();
    account.set_being_liquidated(false);

    Ok(())
}


#[derive(Accounts)]
#[instruction(account_num: u32, token_count: u8)]
pub struct AccountCreate<'info> {
    #[account(
        constraint = market.load()?.is_ix_enabled(IxGate::AccountCreate) @ PeachError::IxIsDisabled,
    )]
    pub market: AccountLoader<'info, Market>,

    #[account(
        init,
        seeds = [b"PeachAccount".as_ref(), market.key().as_ref(), owner.key().as_ref(), &account_num.to_le_bytes()],
        bump,
        payer = payer,
        space = PeachAccountFixed::SIZE,
    )]
    pub account: AccountLoader<'info, PeachAccountFixed>,
    pub owner: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}
