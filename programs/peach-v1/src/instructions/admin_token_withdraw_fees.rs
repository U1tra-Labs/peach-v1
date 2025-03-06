use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token::Token;
use anchor_spl::token::TokenAccount;

use crate::state::*;
use crate::error::PeachError;

use crate::market_seeds;

pub fn admin_token_withdraw_fees(ctx: Context<AdminTokenWithdrawFees>) -> Result<()> {
    let group = ctx.accounts.market.load()?;
    let mut bank = ctx.accounts.bank.load_mut()?;

    let group_seeds = market_seeds!(group);
    let fees = bank.collected_fees_native.floor().to_num::<u64>() - bank.fees_withdrawn;
    let amount = fees.min(ctx.accounts.vault.amount);
    token::transfer(
        ctx.accounts.transfer_ctx().with_signer(&[group_seeds]),
        amount,
    )?;

    bank.fees_withdrawn += amount;

    Ok(())
}



#[derive(Accounts)]
pub struct AdminTokenWithdrawFees<'info> {
    #[account(
        constraint = market.load()?.is_ix_enabled(IxGate::AdminTokenWithdrawFees) @ PeachError::IxIsDisabled,
        has_one = admin,
    )]
    pub market: AccountLoader<'info, Market>,

    #[account(
        mut,
        has_one = market,
        has_one = vault,
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub token_account: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,

    pub admin: Signer<'info>,
}

impl<'info> AdminTokenWithdrawFees<'info> {
    pub fn transfer_ctx(&self) -> CpiContext<'_, '_, '_, 'info, token::Transfer<'info>> {
        let program = self.token_program.to_account_info();
        let accounts = token::Transfer {
            from: self.vault.to_account_info(),
            to: self.token_account.to_account_info(),
            authority: self.market.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }
}
