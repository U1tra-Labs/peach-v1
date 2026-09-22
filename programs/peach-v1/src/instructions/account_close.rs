use anchor_lang::prelude::*;
use anchor_spl::token::Token;

use crate::error::PeachError;
use crate::state::*;

pub fn account_close(ctx: Context<AccountClose>, force_close: bool) -> Result<()> {
    let account = ctx.accounts.account.load_full_mut()?;

    if !ctx.accounts.market.load()?.is_testing() {
        require!(!force_close, PeachError::SomeError);
    }

    if !force_close {
        require!(!account.fixed.being_liquidated(), PeachError::SomeError);
        for ele in account.all_token_positions() {
            require_eq!(ele.is_active(), false);
        }
    }

    Ok(())
}

#[derive(Accounts)]
pub struct AccountClose<'info> {
    #[account(
        constraint = market.load()?.is_ix_enabled(IxGate::AccountClose) @ PeachError::IxIsDisabled,
    )]
    pub market: AccountLoader<'info, Market>,

    #[account(
        mut,
        has_one = market,
        has_one = owner,
        constraint = account.load()?.is_operational() @ PeachError::AccountIsFrozen,
        close = sol_destination
    )]
    pub account: AccountLoader<'info, PeachAccountFixed>,
    pub owner: Signer<'info>,

    #[account(mut)]
    /// CHECK: target for account rent needs no checks
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

