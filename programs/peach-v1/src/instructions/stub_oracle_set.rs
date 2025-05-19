use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::{error::PeachError, custom_types::fixed_wrapper::FixedWrapper, state::*};

pub fn stub_oracle_set(ctx: Context<StubOracleSet>, price: u64) -> Result<()> {
    let mut oracle = ctx.accounts.oracle.load_mut()?;
    oracle.price = FixedWrapper::new(I80F48::from(price));
    oracle.last_update_ts = Clock::get()?.unix_timestamp;
    oracle.last_update_slot = Clock::get()?.slot;
    Ok(())
}

pub fn stub_oracle_set_test(
    ctx: Context<StubOracleSet>,
    price: u64,
    last_update_slot: u64,
    deviation: u64,
) -> Result<()> {
    let mut oracle = ctx.accounts.oracle.load_mut()?;
    oracle.price = FixedWrapper::new(I80F48::from(price));
    oracle.last_update_ts = Clock::get()?.unix_timestamp;
    oracle.last_update_slot = last_update_slot;
    oracle.deviation = FixedWrapper::new(I80F48::from(deviation));

    Ok(())
}

pub fn stub_oracle_set_price(ctx: Context<StubOracleSet>, price: FixedWrapper) -> Result<()> {
    let mut oracle = ctx.accounts.oracle.load_mut()?;
    // TODO: Should we allow setting this to 0?
    oracle.price = price;
    oracle.last_update_ts = Clock::get()?.unix_timestamp;
    oracle.last_update_slot = Clock::get()?.slot;
    Ok(())
}

pub fn stub_oracle_set_all(
    ctx: Context<StubOracleSet>,
    price: FixedWrapper,
    last_update_ts: i64,
    last_update_slot: u64,
    deviation: FixedWrapper,
) -> Result<()> {
    let mut oracle = ctx.accounts.oracle.load_mut()?;
    oracle.price = price;
    oracle.last_update_ts = last_update_ts;
    oracle.last_update_slot = last_update_slot;
    oracle.deviation = deviation;
    Ok(())
}

#[derive(Accounts)]
pub struct StubOracleSet<'info> {
    #[account(
        has_one = admin,
        constraint = market.load()?.is_ix_enabled(IxGate::StubOracleSet) @ PeachError::IxIsDisabled,
    )]
    pub market: AccountLoader<'info, Market>,

    pub admin: Signer<'info>,

    #[account(
        mut,
        has_one = market
    )]
    pub oracle: AccountLoader<'info, StubOracle>,
}

