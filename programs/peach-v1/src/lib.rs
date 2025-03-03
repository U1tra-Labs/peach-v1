use anchor_lang::prelude::*;
use instructions::*;
use state::{OracleConfigParams, TokenIndex};

use fixed::types::I80F48;

#[macro_use]
pub mod util;

pub mod instructions;
pub mod state;
pub mod error;
pub mod i80f48;
pub mod types;
pub mod logs;
pub mod accounts_zerocopy;
pub mod health;

declare_id!("ALBzpjn7Q8T2oiTQc4wgpmFNBr89xtXWcWaGQCpZBzbp");

#[program]
pub mod peach_v1 {

    use super::*;

    pub fn market_create(
        ctx: Context<MarketCreate>, 
        market_num: u32,
        testing: u8,
        version: u8
    ) -> Result<()> {
        instructions::market_create(ctx, market_num, testing, version)
    }

    pub fn account_create(
        ctx: Context<AccountCreate>,
        account_num: u32,
        token_count: u8,
        // token_conditional_swap_count: u8,
        name: String,
    ) -> Result<()> {
        instructions::account_create(
            &ctx.accounts.account,
            ctx.bumps.account,
            ctx.accounts.market.key(),
            ctx.accounts.owner.key(),
            account_num,
            token_count,
            // token_conditional_swap_count,
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

    pub fn stub_oracle_create(ctx: Context<StubOracleCreate>, price: I80F48) -> Result<()> {
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

    pub fn stub_oracle_set(
        ctx: Context<StubOracleSet>,
        price: I80F48,
    ) -> Result<()> {
        instructions::stub_oracle_set(ctx, price)?;
        Ok(())
    }   

    pub fn stub_oracle_close(
        ctx: Context<StubOracleClose>
    ) -> Result<()> {
        instructions::stub_oracle_close(ctx)?;
        Ok(())
    }
}
