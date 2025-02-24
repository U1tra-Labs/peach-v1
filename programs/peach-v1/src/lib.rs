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

declare_id!("EKrZVNcqcS4uJLAq9DuAPP5ewX45XnfaYVy9367uzK2K");

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
        name: String,
    ) -> Result<()> {
        instructions::account_create(
            ctx,
            account_num,
            token_count,
            name,
        )
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
        group_insurance_fund: bool,
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
            group_insurance_fund,
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
}
