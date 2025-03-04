use anchor_lang::prelude::*;

use crate::error::PeachError;
use crate::state::*;
use crate::util::fill_from_str;

// const ONE_WEEK_SECONDS: u64 = 24 * 7 * 60 * 60;

pub fn account_edit(
    ctx: Context<AccountEdit>,
    name_opt: Option<String>,
    // note: can also be used to unset by using the default pubkey here as a param
    delegate_opt: Option<Pubkey>,
    // temporary_delegate_opt: Option<Pubkey>,
    // temporary_delegate_expiry_opt: Option<u64>,
) -> Result<()> {
    require!(
        name_opt.is_some() || delegate_opt.is_some(),
        PeachError::SomeError
    );

    let mut account = ctx.accounts.account.load_full_mut()?;

    if let Some(name) = name_opt {
        account.fixed.name = fill_from_str(&name)?;
    }

    if let Some(delegate) = delegate_opt {
        account.fixed.delegate = delegate;
    }

    // match (temporary_delegate_opt, temporary_delegate_expiry_opt) {
    //     (Some(temporary_delegate), Some(temporary_delegate_expiry)) => {
    //         let now_ts: u64 = Clock::get().unwrap().unix_timestamp.try_into().unwrap();
    //         require_gt!(now_ts + ONE_WEEK_SECONDS, temporary_delegate_expiry);
    //         account.fixed.temporary_delegate = temporary_delegate;
    //         account.fixed.temporary_delegate_expiry = temporary_delegate_expiry;
    //     }
    //     (None, None) => {}
    //     _ => {
    //         return err!(PeachError::SomeError);
    //     }
    // }

    Ok(())
}

#[derive(Accounts)]
pub struct AccountEdit<'info> {
    #[account(
        constraint = market.load()?.is_ix_enabled(IxGate::AccountEdit) @ PeachError::IxIsDisabled,
    )]
    pub market: AccountLoader<'info, Market>,

    #[account(
        mut,
        has_one = market,
        has_one = owner,
        constraint = account.load()?.is_operational() @ PeachError::AccountIsFrozen
    )]
    pub account: AccountLoader<'info, PeachAccountFixed>,
    pub owner: Signer<'info>,
}
