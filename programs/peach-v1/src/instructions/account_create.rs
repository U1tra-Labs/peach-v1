use anchor_lang::prelude::*;

use crate::error::PeachError;
use crate::state::*;

use crate::util::fill_from_str;

pub fn account_create(
    account_ai: &AccountLoader<PeachAccountFixed>,
    account_bump: u8,
    market: Pubkey,
    owner: Pubkey,
    account_num: u32,
    token_count: u8,
    name: String,
) -> Result<()> {
    let mut account = account_ai.load_full_init()?;

    let header = PeachAccountDynamicHeader {
        token_count,
    };
    header.check_resize_from(&PeachAccountDynamicHeader::zero())?;

    msg!(
        "Initialized account with header version {}",
        account.header_version()
    );

    account.fixed.name = fill_from_str(&name)?;
    account.fixed.market = market;
    account.fixed.owner = owner;
    account.fixed.account_num = account_num;
    account.fixed.bump = account_bump;
    account.fixed.delegate = Pubkey::default();
    account.fixed.set_being_liquidated(false);

    account.resize_dynamic_content(
        token_count,
    )?;

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
        space = PeachAccount::space(token_count),
    )]
    pub peach_account: AccountLoader<'info, PeachAccountFixed>,
    pub owner: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}
