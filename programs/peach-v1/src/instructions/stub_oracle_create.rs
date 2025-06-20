use anchor_lang::prelude::*;
use anchor_spl::token::Mint;
use fixed::types::I80F48;

use crate::{error::PeachError, custom_types::fixed_wrapper::FixedWrapper, state::{IxGate, Market, StubOracle}};

pub fn stub_oracle_create(
    ctx: Context<StubOracleCreate>,
    price: u64
) -> Result<()> {
    let mut oracle = ctx.accounts.oracle.load_init()?;
    oracle.market = ctx.accounts.market.key();
    oracle.mint = ctx.accounts.mint.key();
    oracle.price = FixedWrapper::new(I80F48::from(price));
    oracle.last_update_ts = Clock::get()?.unix_timestamp;

    Ok(())
}

#[derive(Accounts)]
pub struct StubOracleCreate<'info> {
    #[account(
        has_one = admin,
        constraint = market.load()?.is_ix_enabled(IxGate::StubOracleCreate) @ PeachError::IxIsDisabled,
    )]
    pub market: AccountLoader<'info, Market>,

    #[account(
        init,
        payer = payer,
        space = 8 + std::mem::size_of::<StubOracle>(),
    )]
    pub oracle: AccountLoader<'info, StubOracle>,

    pub admin: Signer<'info>,

    pub mint: Account<'info, Mint>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}