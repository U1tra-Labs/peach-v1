use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;

// Freezing an account, prevents all instructions involving account (also settling and liquidation)
pub fn account_toggle_freeze(ctx: Context<AccountToggleFreeze>, freeze: bool) -> Result<()> {
    let mut account = ctx.accounts.account.load_full_mut()?;
    if freeze {
        let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();
        account.fixed.frozen_until = now_ts + 7 * 24 * 60 * 60;
    } else {
        account.fixed.frozen_until = 0;
    }

    Ok(())
}

#[derive(Accounts)]
pub struct AccountToggleFreeze<'info> {
    #[account(
        constraint = market.load()?.is_ix_enabled(IxGate::AccountToggleFreeze) @ PeachError::IxIsDisabled,
        constraint = market.load()?.admin == admin.key()
    )]
    pub market: AccountLoader<'info, Market>,

    #[account(
        mut,
        has_one = market,
    )]
    pub account: AccountLoader<'info, PeachAccountFixed>,

    pub admin: Signer<'info>,
}
