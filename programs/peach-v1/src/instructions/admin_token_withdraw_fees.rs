use anchor_lang::prelude::*;
// use anchor_spl::token;
// use anchor_spl::token::Token;
// use anchor_spl::token::TokenAccount;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface};

use crate::state::*;
use crate::error::PeachError;

use crate::market_seeds;

pub fn admin_token_withdraw_fees(ctx: Context<AdminTokenWithdrawFees>) -> Result<()> {
    let market = ctx.accounts.market.load()?;
    let mut bank = ctx.accounts.bank.load_mut()?;

    let market_seeds = market_seeds!(market);
    let fees = bank.collected_fees_native.val().floor().to_num::<u64>() - bank.fees_withdrawn;

    require_gt!(fees, 0);

    let amount = fees.min(ctx.accounts.vault.amount);
    token_interface::transfer_checked(
        ctx.accounts.transfer_ctx().with_signer(&[market_seeds]),
        amount,
        bank.mint_decimals
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
        mint::token_program = token_program,
    )]
    pub mint: InterfaceAccount<'info, Mint>,


    #[account(
        mut,
        has_one = market,
        has_one = vault,
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(mut)]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    #[account(mut)]
    pub token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,

    pub admin: Signer<'info>,
}

impl<'info> AdminTokenWithdrawFees<'info> {
    pub fn transfer_ctx(&self) -> CpiContext<'_, '_, '_, 'info, token_interface::TransferChecked<'info>> {
        let program = self.token_program.to_account_info();
        let accounts = token_interface::TransferChecked {
            from: self.vault.to_account_info(),
            to: self.token_account.to_account_info(),
            authority: self.market.to_account_info(),
            mint: self.mint.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }
}
