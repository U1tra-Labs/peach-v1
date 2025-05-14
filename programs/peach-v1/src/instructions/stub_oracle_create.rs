use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

use crate::{error::PeachError, state::{IxGate, Market, StubOracle, bank::MyFixedIdlWrapper}};

pub fn stub_oracle_create(
    ctx: Context<StubOracleCreate>,
    price: MyFixedIdlWrapper,
    last_update_ts: i64,
    last_update_slot: u64,
    deviation: MyFixedIdlWrapper,
) -> Result<()> {
    let mut oracle = ctx.accounts.oracle.load_init()?;
    oracle.price = price;
    oracle.last_update_ts = last_update_ts;
    oracle.last_update_slot = last_update_slot;
    oracle.deviation = deviation;
    oracle.market = ctx.accounts.market.key();
    oracle.mint = ctx.accounts.mint.key();

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