use crate::accounts_zerocopy::AccountInfoRef;
use crate::constants::KAMINO_PROGRAM_ID;
use crate::error::{Contextable, PeachError};
use crate::health::{
    new_fixed_order_account_retriever_with_optional_banks,
    new_health_cache_skipping_missing_banks_and_bad_oracles,
};
use crate::logs::{emit_stack, LoanOriginationFeeInstruction, WithdrawLoanLog};
use crate::state::{
    oracle_log_context, oracle_state_unchecked, Bank, Market, OracleAccountInfos,
    PeachAccountFixed, PeachAccountLoader,
};
use crate::util::{clock_now, sighash};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::{program::invoke_signed, sysvar};
use anchor_spl::token::Token;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};
use fixed::types::I80F48;

pub fn kamino_withdraw<'info>(
    ctx: Context<'_, '_, '_, 'info, WithdrawKamino<'info>>,
    withdraw_amount: u64,
    allow_borrow: bool,
) -> Result<()> {
    require!(withdraw_amount > 0, PeachError::InvalidAmount);

    msg!("Withdraw amount: {:?}", withdraw_amount);
    msg!("Withdraw mint: {:?}", ctx.accounts.mint.key());

    {
        // let market = ctx.accounts.market.load()?;
        let token_index = ctx.accounts.bank.load()?.token_index;
        let (now_ts, now_slot) = clock_now();

        // Create the account's position for that token index
        let mut account = ctx.accounts.peach_account.load_full_mut()?;
        let (_, raw_token_index, _) = account.ensure_token_position(token_index, 0)?;

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
        let native_position = position.native(&bank);

        // Handle amount special case for withdrawing everything
        let amount = if withdraw_amount == u64::MAX && !allow_borrow {
            if !native_position.is_negative() {
                // TODO: This rounding may mean that if we deposit and immediately withdraw
                //       we can't withdraw the full amount!
                native_position.floor().to_num::<u64>()
            } else {
                return Ok(());
            }
        } else {
            withdraw_amount
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

        // Update the net deposits - adjust by price so different tokens are on the same basis (in USD terms)
        let amount_usd = (amount_i80f48 * unsafe_oracle_state.price).to_num::<i64>();
        account.fixed.net_deposits -= amount_usd;

        //
        // Health check
        //
        if let Some((mut health_cache, pre_init_health_lower_bound)) = pre_health_opt {
            if health_cache.has_token_info(token_index) {
                // This is the normal case: the health cache knows about the token, we can
                // compute the health for the new state by adjusting its balance
                health_cache
                    .adjust_token_balance(&bank, native_position_after - native_position)?;
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
        }

        // emit_stack(WithdrawLog {
        //     peach_market: ctx.accounts.market.key(),
        //     peach_account: ctx.accounts.peach_account.key(),
        //     signer: ctx.accounts.owner.key(),
        //     token_index,
        //     quantity: amount,
        //     price: unsafe_oracle_state.price.to_bits(),
        // });

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

        // TODO: Update peach account: deduct deposit, add withdrawable amount
        // let mut peach_account = ctx.accounts.peach_account.load_mut()?;
        // bank.withdraw_with_fee(&ctx.accounts.mint.key(), withdraw_amount)?;
    }

    msg!("CPI Tranfer: Kamino - Withdraw");

    let (kamino_reserve_liquidity_usdc_supply_pda, _bump) = Pubkey::find_program_address(
        &[
            b"reserve_liq_supply",
            ctx.accounts.lending_market.key().as_ref(),
            ctx.accounts.mint.key().as_ref(),
        ],
        &KAMINO_PROGRAM_ID, // Kamino program ID here!
    );

    require_keys_eq!(
        kamino_reserve_liquidity_usdc_supply_pda,
        ctx.accounts.kamino_reserve_liquidity_usdc_supply.key(),
        PeachError::InvalidKaminoReserveLiquiditySupplyAccount
    );

    let accounts = vec![
        AccountMeta::new(ctx.accounts.signer.key(), true),
        AccountMeta::new(ctx.accounts.obligation.key(), false),
        AccountMeta::new_readonly(ctx.accounts.lending_market.key(), false),
        AccountMeta::new_readonly(ctx.accounts.lending_market_authority.key(), false),
        AccountMeta::new(ctx.accounts.kamino_reserve.key(), false),
        AccountMeta::new_readonly(ctx.accounts.mint.key(), false),
        AccountMeta::new(
            ctx.accounts.kamino_destination_deposit_collateral.key(),
            false,
        ),
        AccountMeta::new(ctx.accounts.kamino_collateral_mint.key(), false),
        AccountMeta::new(
            ctx.accounts.kamino_reserve_liquidity_usdc_supply.key(),
            false,
        ),
        AccountMeta::new(ctx.accounts.user_token_account.key(), false),
        // AccountMeta::new_readonly(
        //     ctx.accounts.user_kamino_reserve_usdc_token_account.key(),
        //     false,
        // ),
        AccountMeta::new_readonly(
            ctx.accounts.kamino_program.key(),
            false,
        ), // placeholder used in klend-sdk
        AccountMeta::new_readonly(ctx.accounts.collateral_token_program.key(), false),
        AccountMeta::new_readonly(ctx.accounts.liquidity_token_program.key(), false),
        AccountMeta::new_readonly(ctx.accounts.instructions_sysvar.key(), false),
        // AccountMeta::new_readonly(ctx.accounts.kamino_program.key(), false),
        AccountMeta::new(ctx.accounts.kamino_obligation_farm_user_state.key(), false),
        AccountMeta::new(ctx.accounts.kamino_reserve_farm_state.key(), false),
        AccountMeta::new_readonly(ctx.accounts.farms_program.key(), false),
    ];

    let discriminator = sighash(
        "global",
        "withdraw_obligation_collateral_and_redeem_reserve_collateral_v2",
    );

    let mut data = discriminator.to_vec();
    data.extend_from_slice(&withdraw_amount.to_le_bytes());

    let kamino_withdraw_ix = Instruction {
        program_id: ctx.accounts.kamino_program.key(),
        accounts,
        data,
    };

    let _market_key = ctx.accounts.market.key();
    let _account_seeds = &ctx.accounts.peach_account.load()?.pda_seeds();

    invoke_signed(
        &kamino_withdraw_ix,
        &[
            ctx.accounts.signer.to_account_info(),
            ctx.accounts.obligation.clone(),
            ctx.accounts.lending_market.clone(),
            ctx.accounts.lending_market_authority.clone(),
            ctx.accounts.kamino_reserve.clone(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.kamino_destination_deposit_collateral.clone(),
            ctx.accounts.kamino_collateral_mint.to_account_info(),
            ctx.accounts.kamino_reserve_liquidity_usdc_supply.clone(),
            ctx.accounts.user_token_account.to_account_info(),
            // ctx.accounts
            //     .user_kamino_reserve_usdc_token_account
            //     .to_account_info(),
            ctx.accounts.kamino_program.to_account_info(),
            ctx.accounts.collateral_token_program.to_account_info(),
            ctx.accounts.liquidity_token_program.to_account_info(),
            ctx.accounts.instructions_sysvar.to_account_info(),
            // ctx.accounts.kamino_program.clone(),
            ctx.accounts.kamino_obligation_farm_user_state.clone(),
            ctx.accounts.kamino_reserve_farm_state.clone(),
            ctx.accounts.farms_program.clone(),
        ],
        &[],
        // &[&account_seeds.signer_seeds()],
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct WithdrawKamino<'info> {
    #[account(
        mut,
        address = peach_account.load()?.owner,
    )]
    pub signer: Signer<'info>,

    /// CHECK: Verified by Kamino program
    #[account(mut)]
    pub obligation: AccountInfo<'info>,

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

    pub market: AccountLoader<'info, Market>,

    #[account(mut)]
    pub kamino_collateral_mint: InterfaceAccount<'info, Mint>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = kamino_collateral_mint,
        associated_token::authority = peach_account,
        associated_token::token_program = collateral_token_program,
    )]
    pub user_kamino_reserve_usdc_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = mint,
        token::authority = signer,
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(mut)]
    /// CHECK: Verified by Kamino program
    pub kamino_reserve: AccountInfo<'info>,
    /// CHECK: Verified by Kamino program
    pub lending_market: AccountInfo<'info>,
    /// CHECK: Verified via PDA constraints
    pub lending_market_authority: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: Verified by Kamino program / process method
    pub kamino_destination_deposit_collateral: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: Verified by Kamino program / process method
    pub kamino_reserve_liquidity_usdc_supply: AccountInfo<'info>,

    #[account(address = KAMINO_PROGRAM_ID)]
    /// CHECK: Kamino program ID
    pub kamino_program: AccountInfo<'info>,
    /// Kamino Farms program
    /// CHECK: Verified by Kamino program
    pub farms_program: AccountInfo<'info>,
    
    #[account(mut)]
    /// CHECK: Verified by Kamino program
    pub kamino_obligation_farm_user_state: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: Verified by Kamino program
    pub kamino_reserve_farm_state: AccountInfo<'info>,

    pub collateral_token_program: Program<'info, Token>,

    pub liquidity_token_program: Interface<'info, TokenInterface>,

    pub system_program: Program<'info, System>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    #[account(address = sysvar::instructions::ID)]
    /// CHECK: This is sysvar instructions account.
    pub instructions_sysvar: UncheckedAccount<'info>,
}
