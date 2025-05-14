use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::accounts_zerocopy::AccountInfoRef;
use crate::error::PeachError;
use crate::instructions::INDEX_START;
use crate::state::*;
use crate::util::fill_from_str;

use crate::logs::{emit_stack, TokenMetaDataLogV2};
use anchor_spl::token::{Mint, Token, TokenAccount};

const FIRST_BANK_NUM: u32 = 0;

#[allow(clippy::too_many_arguments)]
pub fn token_register_trustless(
    ctx: Context<TokenRegisterTrustless>,
    token_index: TokenIndex,
    name: String,
) -> Result<()> {
    require_neq!(token_index, QUOTE_TOKEN_INDEX);
    require_neq!(token_index, TokenIndex::MAX);

    let now_ts: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();

    let net_borrow_limit_window_size_ts = 24 * 60 * 60u64;

    let mut bank = ctx.accounts.bank.load_init()?;
    *bank = Bank {
        market: ctx.accounts.market.key(),
        name: fill_from_str(&name)?,
        mint: ctx.accounts.mint.key(),
        vault: ctx.accounts.vault.key(),
        oracle: ctx.accounts.oracle.key(),
        oracle_config: OracleConfig::default(),
        stable_price_model: StablePriceModel::default(),
        deposit_index: INDEX_START.into(),
        borrow_index: INDEX_START.into(),
        indexed_deposits: I80F48::ZERO.into(),
        indexed_borrows: I80F48::ZERO.into(),
        index_last_updated: now_ts,
        bank_rate_last_updated: now_ts,
        avg_utilization: I80F48::ZERO.into(),
        // Interest rate curve
        adjustment_factor: I80F48::from_num(0.004).into(),
        util0: I80F48::from_num(0.5).into(),
        rate0: I80F48::from_num(0.018).into(),
        util1: I80F48::from_num(0.75).into(),
        rate1: I80F48::from_num(0.05).into(),
        max_rate: I80F48::from_num(0.5).into(),
        collected_fees_native: I80F48::ZERO.into(),
        loan_origination_fee_rate: I80F48::from_num(0.0020).into(),
        loan_fee_rate: I80F48::from_num(0.005).into(),
        maint_asset_weight: I80F48::from_num(0).into(), // Disabled
        init_asset_weight: I80F48::from_num(0).into(),  // Disabled
        maint_liab_weight: I80F48::from_num(1.4).into(), // 2.5x
        init_liab_weight: I80F48::from_num(1.8).into(),  // 1.25x
        liquidation_fee: I80F48::from_num(0.05).into(),
        platform_liquidation_fee: I80F48::from_num(0.05).into(),
        dust: I80F48::ZERO.into(),
        token_index,
        bump: ctx.bumps.bank,
        mint_decimals: ctx.accounts.mint.decimals,
        bank_num: 0,
        min_vault_to_deposits_ratio: F64Bytes::new(0.2),
        net_borrow_limit_window_size_ts,
        last_net_borrows_window_start_ts: now_ts / net_borrow_limit_window_size_ts
            * net_borrow_limit_window_size_ts,
        net_borrow_limit_per_window_quote: 5_000_000_000, // $5k
        net_borrows_in_window: 0,
        borrow_weight_scale_start_quote: F64Bytes::new(5_000_000_000.0), // $5k
        deposit_weight_scale_start_quote: F64Bytes::new(5_000_000_000.0), // $5k
        reduce_only: 0, // Normal mode
        force_close: 0, // Not force close mode
        disable_asset_liquidation: 0, // asset liquidation enabled
        force_withdraw: 0, // force withdraw disabled
        fees_withdrawn: 0,
        interest_target_utilization: F32Bytes::new(0.75), // Corresponds to util1
        interest_curve_scaling: F64Bytes::new(1.0),       // No scaling
        maint_weight_shift_start: 0,
        maint_weight_shift_end: 0,
        maint_weight_shift_duration_inv: I80F48::ZERO.into(),
        maint_weight_shift_asset_target: I80F48::ZERO.into(),
        maint_weight_shift_liab_target: I80F48::ZERO.into(),
        fallback_oracle: ctx.accounts.fallback_oracle.key(),
        deposit_limit: 0, // No limit
        zero_util_rate: I80F48::ZERO.into(),
        collected_liquidation_fees: I80F48::ZERO.into(),
        collected_collateral_fees: I80F48::ZERO.into(),
        collateral_fee_per_day: F32Bytes::new(0.0),
        tier: fill_from_str("C")?,
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

    let mut mint_info = ctx.accounts.mint_info.load_init()?;
    *mint_info = MintInfo {
        market: ctx.accounts.market.key(),
        token_index,
        market_insurance_fund: 0,
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
pub struct TokenRegisterTrustless<'info> {
    #[account(
        mut,
        constraint = market.load()?.admin == admin.key(),
        constraint = market.load()?.is_ix_enabled(IxGate::TokenRegisterTrustless) @ PeachError::IxIsDisabled,
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
        init_if_needed,
        seeds = [b"Vault".as_ref(), market.key().as_ref(), &token_index.0.to_le_bytes(), &FIRST_BANK_NUM.to_le_bytes()],
        bump,
        token::authority = market,
        token::mint = mint,
        payer = payer
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
