use crate::accounts_zerocopy::AccountInfoRef;
use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token;
use fixed::types::I80F48;

use anchor_spl::associated_token::get_associated_token_address;
use anchor_spl::token::Token;
use anchor_spl::token::TokenAccount;

use crate::logs::{emit_stack, ForceWithdrawLog, TokenBalanceLog};

pub fn token_force_withdraw(ctx: Context<TokenForceWithdraw>) -> Result<()> {
    let market = ctx.accounts.market.load()?;
    let token_index = ctx.accounts.bank.load()?.token_index;
    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();

    let mut bank = ctx.accounts.bank.load_mut()?;
    require!(bank.is_force_withdraw(), PeachError::SomeError);

    let mut account = ctx.accounts.account.load_full_mut()?;

    let withdraw_target = if ctx.accounts.owner_ata_token_account.owner == account.fixed.owner {
        ctx.accounts.owner_ata_token_account.to_account_info()
    } else {
        ctx.accounts.alternate_owner_token_account.to_account_info()
    };

    let (position, raw_token_index) = account.token_position_mut(token_index)?;
    let native_position = position.native(&bank);

    // Check >= to allow calling this on 0 deposits to close the token position
    require_gte!(native_position, I80F48::ZERO);
    let amount = native_position.floor().to_num::<u64>();
    let amount_i80f48 = I80F48::from(amount);

    // Update the bank and position
    let position_is_active = bank.withdraw_without_fee(position, amount_i80f48, now_ts)?;

    // Provide a readable error message in case the vault doesn't have enough tokens
    if ctx.accounts.vault.amount < amount {
        return err!(PeachError::InsufficentBankVaultFunds).with_context(|| {
            format!(
                "bank vault does not have enough tokens, need {} but have {}",
                amount, ctx.accounts.vault.amount
            )
        });
    }

    // Transfer the actual tokens
    let market_seeds = market_seeds!(market);
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.vault.to_account_info(),
                to: withdraw_target.clone(),
                authority: ctx.accounts.market.to_account_info(),
            },
        )
        .with_signer(&[market_seeds]),
        amount,
    )?;

    emit_stack(TokenBalanceLog {
        peach_market: ctx.accounts.market.key(),
        peach_account: ctx.accounts.account.key(),
        token_index: token_index.0,
        indexed_position: position.indexed_position.val().to_bits(),
        deposit_index: bank.deposit_index.val().to_bits(),
        borrow_index: bank.borrow_index.val().to_bits(),
    });

    // Get the oracle price, even if stale or unconfident: We want to allow force withdraws
    // even if the oracle is bad.
    let oracle_ref = &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?;
    let unsafe_oracle_state = oracle_state_unchecked(
        &OracleAccountInfos::from_reader(oracle_ref),
        bank.mint_decimals,
    )?;

    // Update the net deposits - adjust by price so different tokens are on the same basis (in USD terms)
    let amount_usd = (amount_i80f48 * unsafe_oracle_state.price).to_num::<i64>();
    account.fixed.net_deposits -= amount_usd;

    if !position_is_active {
        account.deactivate_token_position_and_log(raw_token_index, ctx.accounts.account.key());
    }

    emit_stack(ForceWithdrawLog {
        peach_market: ctx.accounts.market.key(),
        peach_account: ctx.accounts.account.key(),
        token_index: token_index.0,
        quantity: amount,
        price: unsafe_oracle_state.price.to_bits(),
        to_token_account: withdraw_target.key(),
    });

    bank.enforce_borrows_lte_deposits()?;

    Ok(())
}

#[derive(Accounts)]
pub struct TokenForceWithdraw<'info> {
    #[account(
        constraint = market.load()?.is_ix_enabled(IxGate::TokenForceWithdraw) @ PeachError::IxIsDisabled,
    )]
    pub market: AccountLoader<'info, Market>,

    #[account(
        mut,
        has_one = market,
        constraint = account.load()?.is_operational() @ PeachError::AccountIsFrozen,
    )]
    pub account: AccountLoader<'info, PeachAccountFixed>,

    #[account(
        mut,
        has_one = market,
        has_one = vault,
        has_one = oracle,
        // the mints of bank/vault/token_accounts are implicitly the same because
        // spl::token::transfer succeeds between token_account and vault
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(mut)]
    pub vault: Box<Account<'info, TokenAccount>>,

    /// CHECK: The oracle can be one of several different account types
    pub oracle: UncheckedAccount<'info>,

    #[account(
        mut,
        address = get_associated_token_address(&account.load()?.owner, &vault.mint),
        // NOTE: the owner may have been changed (before immutable owner was a thing)
    )]
    pub owner_ata_token_account: Box<Account<'info, TokenAccount>>,

    /// Only for the unusual case where the owner_ata account is not owned by account.owner
    #[account(
        mut,
        constraint = alternate_owner_token_account.owner == account.load()?.owner,
    )]
    pub alternate_owner_token_account: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}
