use anchor_lang::prelude::*;
use anchor_spl::token::Token;

use crate::{error::PeachError, state::*};

pub fn stub_oracle_close(_ctx: Context<StubOracleClose>) -> Result<()> {
    Ok(())
}

#[derive(Accounts)]
pub struct StubOracleClose<'info> {
    #[account(
        has_one = admin,
        constraint = market.load()?.is_ix_enabled(IxGate::StubOracleClose) @ PeachError::IxIsDisabled,
        constraint = market.load()?.is_testing(),
    )]
    pub market: AccountLoader<'info, Market>,
    pub admin: Signer<'info>,

    // match stub oracle to group
    #[account(
        mut,
        has_one = market,
        close = sol_destination
    )]
    pub oracle: AccountLoader<'info, StubOracle>,

    #[account(mut)]
    /// CHECK: target for account rent needs no checks
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}
