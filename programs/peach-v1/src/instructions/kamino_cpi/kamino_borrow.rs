use crate::accounts_zerocopy::AccountInfoRef;
use crate::constants::KAMINO_PROGRAM_ID;
use crate::error::{Contextable, PeachError};
use crate::health::{
    new_fixed_order_account_retriever_with_optional_banks,
    new_health_cache_skipping_missing_banks_and_bad_oracles,
};
use crate::logs::{emit_stack, LoanOriginationFeeInstruction, WithdrawLoanLog};
use crate::state::{
    oracle_log_context, oracle_state_unchecked, Bank, IxGate, Market, OracleAccountInfos,
    PeachAccountFixed, PeachAccountLoader,
};
use crate::util::{clock_now, sighash};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::sysvar;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};
use fixed::types::I80F48;

#[cfg(feature="mainnet")]
use anchor_lang::solana_program::program::invoke;

pub fn kamino_borrow<'info>(
    ctx: Context<'_, '_, '_, 'info, BorrowKamino<'info>>,
    liquidity_amount: u64,
    allow_borrow: bool,
) -> Result<()> {
    require!(liquidity_amount > 0, PeachError::InvalidAmount);

    msg!("Borrow amount: {:?}", liquidity_amount);
    msg!(
        "Borrow mint: {:?}",
        ctx.accounts.borrow_reserve_liquidity_mint.key()
    );

    {
        let token_index = ctx.accounts.bank.load()?.token_index;
        let (now_ts, now_slot) = clock_now();

        // Create the account's position for that token index
        let mut account: crate::state::DynamicAccount<
            crate::state::PeachAccountDynamicHeader,
            std::cell::RefMut<'_, PeachAccountFixed>,
            std::cell::RefMut<'_, [u8]>,
        > = ctx.accounts.peach_account.load_full_mut()?;
        let (token_position, raw_token_index, _) = account.ensure_token_position(token_index, 1)?;

        msg!("Token position: {:?}", token_position);

        // Health check _after_ the token position is guaranteed to exist
        let pre_health_opt = if !account.fixed.is_in_health_region() {
            let retriever = new_fixed_order_account_retriever_with_optional_banks(
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
        let kamino_position = position.native(&bank);

        // Handle amount special case for withdrawing everything
        let amount = if liquidity_amount == u64::MAX && !allow_borrow {
            if !kamino_position.is_negative() {
                // TODO: This rounding may mean that if we deposit and immediately withdraw
                //       we can't withdraw the full amount!
                kamino_position.floor().to_num::<u64>()
            } else {
                return Ok(());
            }
        } else {
            liquidity_amount
        };

        msg!("Amount to borrow: {:?}", amount);

        let is_borrow = amount > kamino_position;
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
        let position_after = position.native(&bank);

        msg!("Kamino position after borrow: {:?}", position_after);

        // Avoid getting in trouble because of the mutable bank account borrow later
        drop(bank);
        let bank = ctx.accounts.bank.load()?;

        // Update the net deposits - adjust by price so different tokens are on the same basis (in USD terms)
        let amount_usd = (amount_i80f48 * unsafe_oracle_state.price).to_num::<i64>();
        account.fixed.net_deposits_kamino -= amount_usd;

        msg!("Net deposits after borrow: {:?}", account.fixed.net_deposits_kamino);

        //
        // Health check
        //
        if let Some((mut health_cache, pre_init_health_lower_bound)) = pre_health_opt {
            if health_cache.has_token_info(token_index) {
                // This is the normal case: the health cache knows about the token, we can
                // compute the health for the new state by adjusting its balance
                health_cache.adjust_token_balance(&bank, position_after - kamino_position)?;
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
            account.deactivate_token_position_and_log(
                raw_token_index,
                ctx.accounts.peach_account.key(),
            );
            msg!("Deactivated token position: {:?}", raw_token_index);
        }

        if withdraw_result.loan_origination_fee.is_positive() {
            emit_stack(WithdrawLoanLog {
                peach_market: ctx.accounts.market.key(),
                peach_account: ctx.accounts.peach_account.key(),
                token_index: token_index.0,
                loan_amount: withdraw_result.loan_amount.to_bits(),
                loan_origination_fee: withdraw_result.loan_origination_fee.to_bits(),
                instruction: LoanOriginationFeeInstruction::TokenWithdraw,
                price: Some(unsafe_oracle_state.price.to_bits()),
            });
        }

        // Enforce min vault to deposits ratio and net borrow limits
        if is_borrow {
            // bank.enforce_max_utilization_on_borrow()?; // Skipped as kamino provides the liquidity

            // When borrowing the price has be trustworthy, so we can do a reasonable
            // net borrow check.
            let now_opt: Option<(u64, u64)> = Some(Clock::get().map(|c| (c.unix_timestamp as u64, c.slot as u64))?);
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

        // TODO: Update peach account: deduct deposit, add withdrawable amount
        // let mut peach_account = ctx.accounts.peach_account.load_mut()?;
        // bank.withdraw_with_fee(&ctx.accounts.mint.key(), withdraw_amount)?;
    }

    msg!("CPI Tranfer: Kamino - Borrow");

    let accounts = vec![
        AccountMeta::new(ctx.accounts.owner.key(), true), // owner
        AccountMeta::new(ctx.accounts.obligation.key(), false), // obligation
        AccountMeta::new_readonly(ctx.accounts.lending_market.key(), false), // lending_market
        AccountMeta::new_readonly(ctx.accounts.lending_market_authority.key(), false), // lending_market_authority
        AccountMeta::new(ctx.accounts.borrow_reserve.key(), false), // borrow_reserve
        AccountMeta::new_readonly(ctx.accounts.borrow_reserve_liquidity_mint.key(), false), // borrow_reserve_liquidity_mint
        AccountMeta::new(ctx.accounts.reserve_source_liquidity.key(), false), // reserve_source_liquidity
        AccountMeta::new(ctx.accounts.borrow_reserve_liquidity_fee_receiver.key(), false,), // borrow_reserve_liquidity_fee_receiver
        AccountMeta::new(ctx.accounts.user_destination_liquidity.key(), false), //         // user_destination_liquidity
        AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),     // token_program
        AccountMeta::new_readonly(ctx.accounts.instruction_sysvar_account.key(), false), // instruction_sysvar_account
        AccountMeta::new(ctx.accounts.obligation_farm_user_state.key(), false), // obligation_farm_user_state
        AccountMeta::new(ctx.accounts.reserve_farm_state.key(), false), // reserve_farm_state
        AccountMeta::new_readonly(ctx.accounts.farms_program.key(), false),    // farms_program
    ];

    let discriminator = sighash("global", "borrow_obligation_liquidity_v2");

    let mut data = discriminator.to_vec();
    data.extend_from_slice(&liquidity_amount.to_le_bytes());

    let kamino_borrow_ix = Instruction {
        program_id: KAMINO_PROGRAM_ID,
        accounts,
        data,
    };

    let account_info = &[
        ctx.accounts.owner.to_account_info(),
        ctx.accounts.obligation.clone(),
        ctx.accounts.lending_market.clone(),
        ctx.accounts.lending_market_authority.clone(),
        ctx.accounts.borrow_reserve.clone(),
        ctx.accounts.borrow_reserve_liquidity_mint.to_account_info(),
        ctx.accounts.reserve_source_liquidity.to_account_info(),
        ctx.accounts.borrow_reserve_liquidity_fee_receiver.to_account_info(),
        ctx.accounts.user_destination_liquidity.to_account_info(),
        ctx.accounts.token_program.to_account_info(), 
        ctx.accounts.instruction_sysvar_account.to_account_info(),
        ctx.accounts.obligation_farm_user_state.clone(),
        ctx.accounts.reserve_farm_state.clone(),
        ctx.accounts.farms_program.clone(),
        ctx.accounts.kamino_program.clone(),
    ];

    #[cfg(feature = "mainnet")]
    {
        invoke(&kamino_borrow_ix, account_info)?;
    }

    #[cfg(not(feature = "mainnet"))]
    {
        msg!("skipped CPI invoke for testnet/devnet");
    }

    Ok(())
}

#[derive(Accounts)]
pub struct BorrowKamino<'info> {
    #[account(
        constraint = market.load()?.is_ix_enabled(IxGate::TokenDeposit) @ PeachError::IxIsDisabled,
    )]
    pub market: AccountLoader<'info, Market>,

    #[account(
        mut,
        has_one = market,
    )]
    pub peach_account: AccountLoader<'info, PeachAccountFixed>,

    #[account(
        mut,
        has_one = market,
        // has_one = vault,
        has_one = oracle,
        // the mints of bank/vault/token_account are implicitly the same because
        // spl::token::transfer succeeds between token_account and vault
    )]
    pub bank: AccountLoader<'info, Bank>,

    // #[account(mut)]
    // pub vault: Account<'info, TokenAccount>,
    /// CHECK: The oracle can be one of several different account types
    pub oracle: UncheckedAccount<'info>,

    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: Verified by Kamino program
    #[account(mut)]
    pub obligation: AccountInfo<'info>,

    /// CHECK: Verified by Kamino program
    pub lending_market: AccountInfo<'info>,
    /// CHECK: Verified via PDA constraints
    pub lending_market_authority: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: Verified by Kamino program
    pub borrow_reserve: AccountInfo<'info>,

    pub borrow_reserve_liquidity_mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    /// CHECK: Verified by Kamino program / process method
    pub reserve_source_liquidity: InterfaceAccount<'info, TokenAccount>,

    #[account(mut)]
    pub borrow_reserve_liquidity_fee_receiver: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = borrow_reserve_liquidity_mint,
        token::authority = owner,
    )]
    pub user_destination_liquidity: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,

    #[account(address = sysvar::instructions::ID)]
    /// CHECK: This is sysvar instructions account.
    pub instruction_sysvar_account: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Verified by Kamino program
    pub obligation_farm_user_state: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: Verified by Kamino program
    pub reserve_farm_state: AccountInfo<'info>,

    /// Kamino Farms program
    /// CHECK: Verified by Kamino program
    pub farms_program: AccountInfo<'info>,

    #[account(address = KAMINO_PROGRAM_ID)]
    /// CHECK: Kamino program ID
    pub kamino_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,

    pub associated_token_program: Program<'info, AssociatedToken>,
}
