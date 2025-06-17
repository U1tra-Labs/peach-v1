use anchor_lang::prelude::*;
use anchor_spl::token_interface;
// use anchor_spl::token;
// use anchor_spl::token::Token;
// use anchor_spl::token::TokenAccount;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::error::*;
use crate::state::*;

use crate::accounts_zerocopy::*;
use crate::health::*;
use crate::util::clock_now;
use fixed::types::I80F48;

use crate::logs::{
    emit_stack, LoanOriginationFeeInstruction, TokenBalanceLog, WithdrawLoanLog, WithdrawLog,
};

pub fn token_withdraw(ctx: Context<TokenWithdraw>, amount: u64, allow_borrow: bool) -> Result<()> {
    require_msg!(amount > 0, "withdraw amount must be positive");

    let market = ctx.accounts.market.load()?;
    let token_index = ctx.accounts.bank.load()?.token_index;
    let (now_ts, now_slot) = clock_now();

    // Create the account's position for that token index
    let mut account = ctx.accounts.account.load_full_mut()?;
    let (_, raw_token_index, _) = account.ensure_token_position(token_index, 0)?;

    // Health check _after_ the token position is guaranteed to exist
    let pre_health_opt = if !account.fixed.is_in_health_region() {
        let retriever: FixedOrderAccountRetriever<AccountInfoRef<'_, '_>> = new_fixed_order_account_retriever_with_optional_banks(
            ctx.remaining_accounts,
            &account.borrow(),
            (now_ts, now_slot),
        )?;
        let health_cache = new_health_cache_skipping_missing_banks_and_bad_oracles(
            &account.borrow(),
            &retriever,
            now_ts,
        )
        .context("pre-withdraw health cache")?;
        let pre_init_health = account.check_health_pre(&health_cache)?;
        Some((health_cache, pre_init_health))
    } else {
        None
    };

    let mut bank = ctx.accounts.bank.load_mut()?;
    let position = account.token_position_mut_by_raw_index(raw_token_index);
    let native_position = position.native(&bank);

    // Handle amount special case for withdrawing everything
    let amount = if amount == u64::MAX && !allow_borrow {
        if !native_position.is_negative() {
            // TODO: This rounding may mean that if we deposit and immediately withdraw
            //       we can't withdraw the full amount!
            native_position.floor().to_num::<u64>()
        } else {
            return Ok(());
        }
    } else {
        amount
    };

    let is_borrow = amount > native_position;
    require!(allow_borrow || !is_borrow, PeachError::SomeError);
    if bank.are_borrows_reduce_only() {
        require!(!is_borrow, PeachError::TokenInReduceOnlyMode);
    }

    let amount_i80f48 = I80F48::from(amount);

    // Get the oracle price, even if stale or unconfident: We want to allow users
    // to withdraw deposits (while staying healthy otherwise) if the oracle is bad.
    let oracle_ref = &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?;
    let unsafe_oracle_state = oracle_state_unchecked(
        &OracleAccountInfos::from_reader(oracle_ref),
        bank.mint_decimals,
    )?;

    // Update the bank and position
    let withdraw_result = bank.withdraw_with_fee(
        position,
        amount_i80f48,
        Clock::get()?.unix_timestamp.try_into().unwrap(),
    )?;
    let native_position_after = position.native(&bank);

    // Avoid getting in trouble because of the mutable bank account borrow later
    drop(bank);
    let bank = ctx.accounts.bank.load()?;

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
    token_interface::transfer_checked(
        ctx.accounts.transfer_ctx().with_signer(&[market_seeds]),
        amount,
        bank.mint_decimals,
    )?;

    emit_stack(TokenBalanceLog {
        peach_market: ctx.accounts.market.key(),
        peach_account: ctx.accounts.account.key(),
        token_index: token_index.0,
        indexed_position: position.indexed_position.val().to_bits(),
        deposit_index: bank.deposit_index.val().to_bits(),
        borrow_index: bank.borrow_index.val().to_bits(),
    });

    // Update the net deposits - adjust by price so different tokens are on the same basis (in USD terms)
    let amount_usd = (amount_i80f48 * unsafe_oracle_state.price).to_num::<i64>();
    account.fixed.net_deposits -= amount_usd;

    // // Delegates have heavy restrictions on withdraws. #1
    // if account.fixed.is_delegate(ctx.accounts.owner.key()) {
    //     // Delegates can only withdrawing into the actual owner's ATA
    //     let owner_ata = associated_token::get_associated_token_address(
    //         &account.fixed.owner,
    //         &ctx.accounts.vault.mint,
    //     );
    //     require_keys_eq!(
    //         ctx.accounts.token_account.key(),
    //         owner_ata,
    //         PeachError::DelegateWithdrawOnlyToOwnerAta
    //     );
    //     require_keys_eq!(
    //         ctx.accounts.token_account.owner,
    //         account.fixed.owner,
    //         PeachError::DelegateWithdrawOnlyToOwnerAta
    //     );

    //     // Delegates must close the token position
    //     require!(
    //         !withdraw_result.position_is_active,
    //         PeachError::DelegateWithdrawMustClosePosition
    //     );
    // }

    //
    // Health check
    //
    if let Some((mut health_cache, pre_init_health_lower_bound)) = pre_health_opt {
        if health_cache.has_token_info(token_index) {
            // This is the normal case: the health cache knows about the token, we can
            // compute the health for the new state by adjusting its balance
            health_cache.adjust_token_balance(&bank, native_position_after - native_position)?;
            account.check_health_post(&health_cache, pre_init_health_lower_bound)?;
        } else {
            // The health cache does not know about the token! It has a bad oracle or wasn't
            // provided in the health accounts. Borrows are out of the question!
            require!(!is_borrow, PeachError::BorrowsRequireHealthAccountBank);

            // Since the health cache isn't aware of the bank we changed, the health
            // estimation is the same.
            let post_init_health_lower_bound = pre_init_health_lower_bound;

            // If health without the token is positive, then full health is positive and
            // withdrawing all of the token would still keep it positive.
            // However, if health without it is negative then full health could be negative
            // and could be made worse by withdrawals.
            //
            // We don't know the true pre_init_health: So require that our lower bound on
            // post health is strictly good enough.
            account.check_health_post_checks_strict(post_init_health_lower_bound)?;
        }
    }

    //
    // Deactivate the position only after the health check because the user passed in
    // remaining_accounts for all banks/oracles, including the account that will now be
    // deactivated.
    //
    if !withdraw_result.position_is_active {
        account.deactivate_token_position_and_log(raw_token_index, ctx.accounts.account.key());
    }

    emit_stack(WithdrawLog {
        peach_market: ctx.accounts.market.key(),
        peach_account: ctx.accounts.account.key(),
        signer: ctx.accounts.owner.key(),
        token_index: token_index.0,
        quantity: amount,
        price: unsafe_oracle_state.price.to_bits(),
    });

    if withdraw_result.loan_origination_fee.is_positive() {
        emit_stack(WithdrawLoanLog {
            peach_market: ctx.accounts.market.key(),
            peach_account: ctx.accounts.account.key(),
            token_index: token_index.0,
            loan_amount: withdraw_result.loan_amount.to_bits(),
            loan_origination_fee: withdraw_result.loan_origination_fee.to_bits(),
            instruction: LoanOriginationFeeInstruction::TokenWithdraw,
            price: Some(unsafe_oracle_state.price.to_bits()),
        });
    }

    // Enforce min vault to deposits ratio and net borrow limits
    if is_borrow {
        bank.enforce_max_utilization_on_borrow()?;

        // When borrowing the price has be trustworthy, so we can do a reasonable
        // net borrow check.
        let now_opt = Some(Clock::get().map(|c| (c.unix_timestamp as u64, c.slot as u64))?);
        unsafe_oracle_state
            .check_confidence_and_maybe_staleness(&bank.oracle_config, now_opt)
            .with_context(|| {
                oracle_log_context(
                    bank.name(),
                    &unsafe_oracle_state,
                    &bank.oracle_config,
                    now_opt,
                )
            })?;
        bank.check_net_borrows(unsafe_oracle_state.price)?;
    } else {
        bank.enforce_borrows_lte_deposits()?;
    }

    Ok(())
}

#[derive(Accounts)]
pub struct TokenWithdraw<'info> {
    #[account(
        constraint = market.load()?.is_ix_enabled(IxGate::TokenWithdraw) @ PeachError::IxIsDisabled,
    )]
    pub market: AccountLoader<'info, Market>,

    #[account(
        mut,
        has_one = market,
        constraint = account.load()?.is_operational() @ PeachError::AccountIsFrozen,
        constraint = account.load()?.is_owner(owner.key()),
    )]
    pub account: AccountLoader<'info, PeachAccountFixed>,
    pub owner: Signer<'info>,

    #[account(
        mint::token_program = token_program,
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        has_one = market,
        has_one = vault,
        has_one = oracle,
        // the mints of bank/vault/token_account are implicitly the same because
        // spl::token::transfer succeeds between token_account and vault
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(mut)]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: The oracle can be one of several different account types
    pub oracle: UncheckedAccount<'info>,

    #[account(mut)]
    pub token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> TokenWithdraw<'info> {
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
