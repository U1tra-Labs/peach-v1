use crate::error::PeachError;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::Token;

pub fn market_close(_ctx: Context<MarketClose>) -> Result<()> {
    Ok(())
}

#[derive(Accounts)]
pub struct MarketClose<'info> {
    #[account(
        mut,
        has_one = admin,
        constraint = market.load()?.is_testing(),
        constraint = market.load()?.is_ix_enabled(IxGate::MarketClose) @ PeachError::IxIsDisabled,
        close = sol_destination
    )]
    pub market: AccountLoader<'info, Market>,

    pub admin: Signer<'info>,

    #[account(mut)]
    /// CHECK: target for account rent needs no checks
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}
