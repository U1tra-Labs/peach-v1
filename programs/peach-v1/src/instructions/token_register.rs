use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::accounts_zerocopy::AccountInfoRef;
use crate::error::*;
use crate::logs::{emit_stack, TokenMetaDataLogV2};
use crate::state::*;

const FIRST_BANK_NUM: u32 = 0;

use fixed::types::I80F48;

use crate::util::fill_from_str;

// use crate::logs::{emit_stack, TokenMetaDataLogV2};

pub const INDEX_START: I80F48 = I80F48::from_bits(1_000_000 * I80F48::ONE.to_bits());

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
    require_neq!(token_index, TokenIndex::MAX);

    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();

    msg!("Initializing bank account");
    let mut bank = ctx.accounts.bank.load_init()?;
    *bank = Bank {
        market: ctx.accounts.market.key(),
        name: fill_from_str(&name)?,
        mint: ctx.accounts.mint.key(),
        vault: ctx.accounts.vault.key(),
        oracle: ctx.accounts.oracle.key(),
        deposit_index: INDEX_START.into(),
        borrow_index: INDEX_START.into(),
        indexed_deposits: I80F48::ZERO.into(),
        indexed_borrows: I80F48::ZERO.into(),
        index_last_updated: now_ts,
        bank_rate_last_updated: now_ts,
        // TODO: add a require! verifying relation between the parameters
        avg_utilization: I80F48::ZERO.into(),
        adjustment_factor: I80F48::from_num(interest_rate_params.adjustment_factor).into(),
        util0: I80F48::from_num(interest_rate_params.util0).into(),
        rate0: I80F48::from_num(interest_rate_params.rate0).into(),
        util1: I80F48::from_num(interest_rate_params.util1).into(),
        rate1: I80F48::from_num(interest_rate_params.rate1).into(),
        max_rate: I80F48::from_num(interest_rate_params.max_rate).into(),
        collected_fees_native: I80F48::ZERO.into(),
        loan_origination_fee_rate: I80F48::from_num(loan_origination_fee_rate).into(),
        loan_fee_rate: I80F48::from_num(loan_fee_rate).into(),
        maint_asset_weight: I80F48::from_num(maint_asset_weight).into(),
        init_asset_weight: I80F48::from_num(init_asset_weight).into(),
        maint_liab_weight: I80F48::from_num(maint_liab_weight).into(),
        init_liab_weight: I80F48::from_num(init_liab_weight).into(),
        liquidation_fee: I80F48::from_num(liquidation_fee).into(),
        dust: I80F48::ZERO.into(),
        token_index,
        bump: ctx.bumps.bank,
        mint_decimals: ctx.accounts.mint.decimals,
        bank_num: 0,
        oracle_config: oracle_config.to_oracle_config(),
        stable_price_model: StablePriceModel {
            delay_interval_seconds: stable_price_delay_interval_seconds,
            delay_growth_limit: stable_price_delay_growth_limit,
            stable_growth_limit: stable_price_growth_limit,
            ..StablePriceModel::default()
        },
        min_vault_to_deposits_ratio: F64Bytes::new(min_vault_to_deposits_ratio),
        net_borrow_limit_window_size_ts,
        last_net_borrows_window_start_ts: now_ts / net_borrow_limit_window_size_ts
            * net_borrow_limit_window_size_ts,
        net_borrow_limit_per_window_quote,
        net_borrows_in_window: 0,
        borrow_weight_scale_start_quote: F64Bytes::new(borrow_weight_scale_start_quote),
        deposit_weight_scale_start_quote: F64Bytes::new(deposit_weight_scale_start_quote),
        reduce_only,
        force_close: 0,
        disable_asset_liquidation: u8::from(disable_asset_liquidation),
        force_withdraw: 0,
        fees_withdrawn: 0,
        interest_target_utilization: F32Bytes::new(interest_target_utilization),
        interest_curve_scaling: F64Bytes::new(interest_curve_scaling as f64),
        maint_weight_shift_start: 0,
        maint_weight_shift_end: 0,
        maint_weight_shift_duration_inv: I80F48::ZERO.into(),
        maint_weight_shift_asset_target: I80F48::ZERO.into(),
        maint_weight_shift_liab_target: I80F48::ZERO.into(),
        fallback_oracle: ctx.accounts.fallback_oracle.key(),
        deposit_limit,
        zero_util_rate: I80F48::from_num(zero_util_rate).into(),
        platform_liquidation_fee: I80F48::from_num(platform_liquidation_fee).into(),
        collected_liquidation_fees: I80F48::ZERO.into(),
        collected_collateral_fees: I80F48::ZERO.into(),
        collateral_fee_per_day: F32Bytes::new(collateral_fee_per_day),
        tier: fill_from_str(&tier)?,
        _padding2: [0; 4],
        _padding3: [0; 48],
        reserved: [0u8; 4],
    };

    let oracle_ref = &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?;
    if let Ok(oracle_price) = bank.oracle_price(&OracleAccountInfos::from_reader(oracle_ref), None)
    {
        bank.stable_price_model
            .reset_to_price(oracle_price.to_num(), now_ts);
    } else {
        bank.stable_price_model.reset_on_nonzero_price = 1;
    }

    bank.verify()?;
    check_is_valid_fallback_oracle(&AccountInfoRef::borrow(
        ctx.accounts.fallback_oracle.as_ref(),
    )?)?;

    msg!("Initializing mint_info account");
    let mut mint_info = ctx.accounts.mint_info.load_init()?;
    *mint_info = MintInfo {
        market: ctx.accounts.market.key(),
        token_index,
        market_insurance_fund: if market_insurance_fund { 1 } else { 0 },
        padding1: Default::default(),
        mint: ctx.accounts.mint.key(),
        banks: Default::default(),
        vaults: Default::default(),
        oracle: ctx.accounts.oracle.key(),
        fallback_oracle: ctx.accounts.fallback_oracle.key(),
        registration_time: Clock::get()?.unix_timestamp.try_into().unwrap(),
    };

    mint_info.banks[0] = ctx.accounts.bank.key();
    mint_info.vaults[0] = ctx.accounts.vault.key();

    emit_stack(TokenMetaDataLogV2 {
        market: ctx.accounts.market.key(),
        mint: ctx.accounts.mint.key(),
        token_index: token_index.0,
        mint_decimals: ctx.accounts.mint.decimals,
        oracle: ctx.accounts.oracle.key(),
        fallback_oracle: ctx.accounts.fallback_oracle.key(),
        mint_info: ctx.accounts.mint_info.key(),
    });

    Ok(())
}

#[derive(Accounts)]
#[instruction(token_index: TokenIndex)]
pub struct TokenRegister<'info> {
    #[account(
        has_one = admin,
        constraint = market.load()?.is_ix_enabled(IxGate::TokenRegister) @ PeachError::IxIsDisabled,
    )]
    pub market: AccountLoader<'info, Market>,
    pub admin: Signer<'info>,

    pub mint: Account<'info, Mint>,

    #[account(
        init,
        // using the token_index in this seed guards against reusing it
        seeds = [b"Bank".as_ref(), market.key().as_ref(), &token_index.0.to_le_bytes(), &FIRST_BANK_NUM.to_le_bytes()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<Bank>(),
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(
        seeds = [b"Vault".as_ref(), market.key().as_ref(), &token_index.0.to_le_bytes(), &FIRST_BANK_NUM.to_le_bytes()],
        bump,
        token::authority = market,
        token::mint = mint,
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        init,
        // using the mint in this seed guards against registering the same mint twice
        seeds = [b"MintInfo".as_ref(), market.key().as_ref(), mint.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<MintInfo>(),
    )]
    pub mint_info: AccountLoader<'info, MintInfo>,

    /// CHECK: The oracle can be one of several different account types
    pub oracle: UncheckedAccount<'info>,

    /// CHECK: The oracle can be one of several different account types
    pub fallback_oracle: UncheckedAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
#[repr(C)]
pub struct InterestRateParams {
    pub util0: f32,
    pub rate0: f32,
    pub util1: f32,
    pub rate1: f32,
    pub max_rate: f32,
    pub adjustment_factor: f32,
}
