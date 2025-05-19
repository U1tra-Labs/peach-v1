use anchor_lang::prelude::*;
use instructions::*;
use state::OracleConfigParams;
use custom_types::token_index::TokenIndex;

#[macro_use]
pub mod util;

pub mod instructions;
pub mod state;
pub mod error;
pub mod types;
pub mod custom_types;
pub mod logs;
pub mod accounts_zerocopy;
pub mod health;
pub mod constants;

declare_id!("EYz3GPbE1qoXS3i4ziRmfyTQgVjZt3ymMxYKZZiEaEED");

#[program]
pub mod peach_v1 {

    use crate::custom_types::TokenIndex;

    use super::*;

    pub fn market_create(
        ctx: Context<MarketCreate>, 
        market_num: u32,
        testing: u8,
        version: u8
    ) -> Result<()> {
        instructions::market_create(ctx, market_num, testing, version)?;
        Ok(())
    }

    pub fn account_create(
        ctx: Context<AccountCreate>,
        account_num: u32,
        token_count: u8,
        // token_conditional_swap_count: u8,
        name: String,
    ) -> Result<()> {
        instructions::account_create(
            &ctx.accounts.peach_account,
            ctx.bumps.peach_account,
            ctx.accounts.market.key(),
            ctx.accounts.owner.key(),
            account_num,
            token_count,
            name,
        )?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn token_register(
        ctx: Context<TokenRegister>,
        token_index: TokenIndex,
        name: String,
        oracle_config: OracleConfigParams,
        interest_rate_params: InterestRateParams,
        loan_fee_rate: f32,
        loan_origination_fee_rate: f32,
        maint_asset_weight: f32,
        init_asset_weight: f32,
        maint_liab_weight: f32,
        init_liab_weight: f32,
        liquidation_fee: f32,
        stable_price_delay_interval_seconds: u32,
        stable_price_delay_growth_limit: f32,
        stable_price_growth_limit: f32,
        min_vault_to_deposits_ratio: f64,
        net_borrow_limit_window_size_ts: u64,
        net_borrow_limit_per_window_quote: i64,
        borrow_weight_scale_start_quote: f64,
        deposit_weight_scale_start_quote: f64,
        reduce_only: u8,
        interest_curve_scaling: f32,
        interest_target_utilization: f32,
        market_insurance_fund: bool,
        deposit_limit: u64,
        zero_util_rate: f32,
        platform_liquidation_fee: f32,
        disable_asset_liquidation: bool,
        collateral_fee_per_day: f32,
        tier: String,
    ) -> Result<()> {
        instructions::token_register(
            ctx,
            token_index,
            name,
            oracle_config,
            interest_rate_params,
            loan_fee_rate,
            loan_origination_fee_rate,
            maint_asset_weight,
            init_asset_weight,
            maint_liab_weight,
            init_liab_weight,
            liquidation_fee,
            stable_price_delay_interval_seconds,
            stable_price_delay_growth_limit,
            stable_price_growth_limit,
            min_vault_to_deposits_ratio,
            net_borrow_limit_window_size_ts,
            net_borrow_limit_per_window_quote,
            borrow_weight_scale_start_quote,
            deposit_weight_scale_start_quote,
            reduce_only,
            interest_curve_scaling,
            interest_target_utilization,
            market_insurance_fund,
            deposit_limit,
            zero_util_rate,
            platform_liquidation_fee,
            disable_asset_liquidation,
            collateral_fee_per_day,
            tier,
        )?;
        Ok(())
    }

    pub fn token_vault_create(
        ctx: Context<TokenVaultCreate>,
        token_index: TokenIndex,
    ) -> Result<()> {
        instructions::token_vault_create(ctx, token_index)?;
        Ok(())
    }

    pub fn stub_oracle_create(
        ctx: Context<StubOracleCreate>,
        price: u64
    ) -> Result<()> {
        instructions::stub_oracle_create(ctx, price)?;
        Ok(())
    }

    pub fn token_deposit(ctx: Context<TokenDeposit>, amount: u64, reduce_only: bool) -> Result<()> {
        instructions::token_deposit(ctx, amount, reduce_only)?;
        Ok(())
    }

    pub fn token_deposit_into_existing(
        ctx: Context<TokenDepositIntoExisting>,
        amount: u64,
        reduce_only: bool,
    ) -> Result<()> {
        instructions::token_deposit_into_existing(ctx, amount, reduce_only)?;
        Ok(())
    }

    pub fn token_withdraw(
        ctx: Context<TokenWithdraw>,
        amount: u64,
        allow_borrow: bool,
    ) -> Result<()> {
        instructions::token_withdraw(ctx, amount, allow_borrow)?;
        Ok(())
    }

    pub fn token_force_withdraw(ctx: Context<TokenForceWithdraw>) -> Result<()> {
        instructions::token_force_withdraw(ctx)?;
        Ok(())
    }

    pub fn token_charge_collateral_fees(ctx: Context<TokenChargeCollateralFees>) -> Result<()> {
        instructions::token_charge_collateral_fees(ctx)?;
        Ok(())
    }

    pub fn market_edit(
        ctx: Context<MarketEdit>,
        admin_opt: Option<Pubkey>,
        testing_opt: Option<u8>,
        version_opt: Option<u8>,
        deposit_limit_quote_opt: Option<u64>,
        collateral_fee_interval_opt: Option<u64>,
    ) -> Result<()> {
        instructions::market_edit(ctx, admin_opt, testing_opt, version_opt, deposit_limit_quote_opt, collateral_fee_interval_opt)?;
        Ok(())
    }

    pub fn market_close(
        ctx: Context<MarketClose>
    ) -> Result<()> {
        instructions::market_close(ctx)?;
        Ok(())
    }

    pub fn stub_oracle_set(ctx: Context<StubOracleSet>, price: u64) -> Result<()> {
        instructions::stub_oracle_set(ctx, price)?;
        Ok(())
    }

    pub fn stub_oracle_set_test(
        ctx: Context<StubOracleSet>,
        price: u64,
        last_update_slot: u64,
        deviation: u64,
    ) -> Result<()> {
        instructions::stub_oracle_set_test(ctx, price, last_update_slot, deviation)?;
        Ok(())
    } 

    pub fn stub_oracle_close(
        ctx: Context<StubOracleClose>
    ) -> Result<()> {
        instructions::stub_oracle_close(ctx)?;
        Ok(())
    }

    pub fn account_edit(
        ctx: Context<AccountEdit>,
        name_opt: Option<String>,
        delegate_opt: Option<Pubkey>,
    ) -> Result<()> {
        instructions::account_edit(ctx, name_opt, delegate_opt)?;
        Ok(())
    }

    pub fn account_expand(
        ctx: Context<AccountExpand>,
        token_count: u8
    ) -> Result<()> {
        instructions::account_expand(ctx, token_count)?;
        Ok(())
    }

    pub fn account_size_migration(ctx: Context<AccountSizeMigration>) -> Result<()> {
        instructions::account_size_migration(ctx)?;
        Ok(())
    }

    pub fn account_toggle_freeze(ctx: Context<AccountToggleFreeze>, freeze: bool) -> Result<()> {
        instructions::account_toggle_freeze(ctx, freeze)?;
        Ok(())
    }

    pub fn account_close(ctx: Context<AccountClose>, force_close: bool) -> Result<()> {
        instructions::account_close(ctx, force_close)?;
        Ok(())
    }

    pub fn token_register_trustless(ctx: Context<TokenRegisterTrustless>, token_index: TokenIndex, name: String) -> Result<()> {    
        instructions::token_register_trustless(ctx, token_index, name)?;
        Ok(())    
    }

    pub fn token_add_bank(ctx: Context<TokenAddBank>, token_index: TokenIndex, bank_num: u32) -> Result<()> {
        instructions::token_add_bank(ctx, token_index, bank_num)?;
        Ok(())    
    }

    #[allow(clippy::too_many_arguments)]
    pub fn token_edit(
        ctx: Context<TokenEdit>, 
        oracle_opt: Option<Pubkey>,
        oracle_config_opt: Option<OracleConfigParams>,
        interest_rate_params_opt: Option<InterestRateParams>,
        loan_fee_rate_opt: Option<f32>,
        loan_origination_fee_rate_opt: Option<f32>,
        maint_asset_weight_opt: Option<f32>,
        init_asset_weight_opt: Option<f32>,
        maint_liab_weight_opt: Option<f32>,
        init_liab_weight_opt: Option<f32>,
        liquidation_fee_opt: Option<f32>,
        stable_price_delay_interval_seconds_opt: Option<u32>,
        stable_price_delay_growth_limit_opt: Option<f32>,
        stable_price_growth_limit_opt: Option<f32>,
        min_vault_to_deposits_ratio_opt: Option<f64>,
        net_borrow_limit_per_window_quote_opt: Option<i64>,
        net_borrow_limit_window_size_ts_opt: Option<u64>,
        borrow_weight_scale_start_quote_opt: Option<f64>,
        deposit_weight_scale_start_quote_opt: Option<f64>,
        reset_stable_price: bool,
        reset_net_borrow_limit: bool,
        reduce_only_opt: Option<u8>,
        name_opt: Option<String>,
        force_close_opt: Option<bool>,
        token_conditional_swap_taker_fee_rate_opt: Option<f32>,
        token_conditional_swap_maker_fee_rate_opt: Option<f32>,
        flash_loan_swap_fee_rate_opt: Option<f32>,
        interest_curve_scaling_opt: Option<f32>,
        interest_target_utilization_opt: Option<f32>,
        maint_weight_shift_start_opt: Option<u64>,
        maint_weight_shift_end_opt: Option<u64>,
        maint_weight_shift_asset_target_opt: Option<f32>,
        maint_weight_shift_liab_target_opt: Option<f32>,
        maint_weight_shift_abort: bool,
        set_fallback_oracle: bool,
        deposit_limit_opt: Option<u64>,
        zero_util_rate: Option<f32>,
        platform_liquidation_fee: Option<f32>,
        disable_asset_liquidation_opt: Option<bool>,
        collateral_fee_per_day: Option<f32>,
        force_withdraw_opt: Option<bool>,
        tier_opt: Option<String>,
    ) -> Result<()> {
        instructions::token_edit(
            ctx, 
            oracle_opt, 
            oracle_config_opt, 
            interest_rate_params_opt, 
            loan_fee_rate_opt, 
            loan_origination_fee_rate_opt, 
            maint_asset_weight_opt, 
            init_asset_weight_opt, 
            maint_liab_weight_opt, 
            init_liab_weight_opt, 
            liquidation_fee_opt, 
            stable_price_delay_interval_seconds_opt, 
            stable_price_delay_growth_limit_opt, 
            stable_price_growth_limit_opt, 
            min_vault_to_deposits_ratio_opt, 
            net_borrow_limit_per_window_quote_opt, 
            net_borrow_limit_window_size_ts_opt, 
            borrow_weight_scale_start_quote_opt, 
            deposit_weight_scale_start_quote_opt, 
            reset_stable_price, 
            reset_net_borrow_limit, 
            reduce_only_opt, 
            name_opt, 
            force_close_opt, 
            token_conditional_swap_taker_fee_rate_opt, 
            token_conditional_swap_maker_fee_rate_opt, 
            flash_loan_swap_fee_rate_opt, 
            interest_curve_scaling_opt, 
            interest_target_utilization_opt, 
            maint_weight_shift_start_opt, 
            maint_weight_shift_end_opt, 
            maint_weight_shift_asset_target_opt, 
            maint_weight_shift_liab_target_opt, 
            maint_weight_shift_abort, 
            set_fallback_oracle, 
            deposit_limit_opt, 
            zero_util_rate, 
            platform_liquidation_fee, 
            disable_asset_liquidation_opt, 
            collateral_fee_per_day, 
            force_withdraw_opt, 
            tier_opt
        )?;
        Ok(())
    }

    pub fn  token_update_index_and_rate(ctx: Context<TokenUpdateIndexAndRate>) -> Result<()> {
        instructions::token_update_index_and_rate(ctx, false)?;
        Ok(())    
    }

    pub fn token_update_index_and_rate_resilient(
        ctx: Context<TokenUpdateIndexAndRate>,
    ) -> Result<()> {
        instructions::token_update_index_and_rate(ctx, true)?;
        Ok(())
    }

    pub fn token_deregister<'key, 'accounts, 'remaining: 'info, 'info>(
        ctx: Context<'key, 'accounts, 'remaining, 'info, TokenDeregister<'info>>,
    ) -> Result<()> {
        instructions::token_deregister(ctx)?;
        Ok(())
    }

    pub fn admin_token_withdraw_fees(ctx: Context<AdminTokenWithdrawFees>) -> Result<()> {
        instructions::admin_token_withdraw_fees(ctx)?;
        Ok(())
    }

    pub fn ix_gate_set(ctx: Context<IxGateSet>, ix_gate: u128) -> Result<()> {
        instructions::ix_gate_set(ctx, ix_gate)?;
        Ok(())
    }
        
    pub fn kamino_init_user_metadata <'info> (
        ctx: Context<KaminoInitUserMetaData>,
        user_lookup_table_account: Pubkey
    ) -> Result<()> {
        instructions::kamino_init_user_metadata(ctx, user_lookup_table_account)
    }

    pub fn kamino_init_obligation <'info> (
        ctx: Context<KaminoInitObligation>,
        args: InitObligationArgs,
    ) -> Result<()> {
        instructions::kamino_init_obligation(ctx, args)
    }

    pub fn kamino_init_obligation_farm_for_reserve<'info> (
        ctx: Context<KaminoInitObligationFarmsForReserve>,
        mod_num: u8,
    ) -> Result<()> {
        instructions::kamino_init_obligation_farm_for_reserve(ctx, mod_num)
    }

    // Kamino Cpi Call - Init Obligation Farm for Reserve
    pub fn kamino_deposit<'info> (
        ctx: Context<'_, '_, '_, 'info, DepositKamino<'info>>,
        deposit_amount: u64,
        reduce_only: bool,
    ) -> Result<()> {
        instructions::kamino_deposit(ctx, deposit_amount, reduce_only)
    }   

    // Kamino Cpi Call - Withdraw
    pub fn kamino_withdraw<'info> (
        ctx: Context<'_, '_, '_, 'info, WithdrawKamino<'info>>,
        withdraw_amount: u64,
        allow_borrow: bool,
    ) -> Result<()> {
        instructions::kamino_withdraw(ctx, withdraw_amount, allow_borrow)
    }

}
