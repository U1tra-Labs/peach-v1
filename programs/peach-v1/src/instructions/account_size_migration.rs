use anchor_lang::prelude::*;

use crate::state::*;
use crate::error::PeachError;

pub fn account_size_migration(ctx: Context<AccountSizeMigration>) -> Result<()> {
    let account = ctx.accounts.account.load_full()?;
    let current_header = account.header.clone();

    let account_ai = ctx.accounts.account.as_ref();
    let current_size = account_ai.data_len();
    let current_lamports = account_ai.lamports();

    // If the account has the expected size, we were already called on it
    if current_size == current_header.account_size() {
        return Ok(());
    }

    let new_header = PeachAccountDynamicHeader {
        token_count: current_header.token_count.min(
            account
                .active_token_positions()
                .count()
                .max(8)
                .try_into()
                .unwrap(),
        ),
        ..current_header
    };
    drop(account);

    let new_size = new_header.account_size();
    let new_rent_minimum = Rent::get()?.minimum_balance(new_size);

    if current_lamports < new_rent_minimum {
        anchor_lang::system_program::transfer(
            anchor_lang::context::CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.payer.to_account_info(),
                    to: account_ai.clone(),
                },
            ),
            new_rent_minimum - current_lamports,
        )?;
    }
    // If there's too many lamports, they belong to the user and are kept on the account

    // This instruction has <= 1 calls to AccountInfo::realloc(), meaning that the
    // new data when expanding the account will be zero initialized already: it's not
    // necessary to zero init it again.
    let no_zero_init = false;

    if new_size > current_size {
        account_ai.realloc(new_size, no_zero_init)?;
    }

    // resize the dynamic content on the account
    {
        let mut account = ctx.accounts.account.load_full_mut()?;
        account.resize_dynamic_content(new_header.token_count)?;
    }

    if new_size < current_size {
        account_ai.realloc(new_size, no_zero_init)?;
    }

    Ok(())
}

#[derive(Accounts)]
pub struct AccountSizeMigration<'info> {
    #[account(
        constraint = market.load()?.is_ix_enabled(IxGate::AccountSizeMigration) @ PeachError::IxIsDisabled,
    )]
    pub market: AccountLoader<'info, Market>,

    #[account(
        mut,
        has_one = market,
    )]
    pub account: AccountLoader<'info, PeachAccountFixed>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}
