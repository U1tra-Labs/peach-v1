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
        oracle_config: OracleConfig {
            conf_filter: I80F48::from_num(1000.0), // effectively disabled
            max_staleness_slots: -1,
            reserved: [0; 72],
        },
        stable_price_model: StablePriceModel::default(),
        deposit_index: INDEX_START,
        borrow_index: INDEX_START,
        indexed_deposits: I80F48::ZERO,
        indexed_borrows: I80F48::ZERO,
        index_last_updated: now_ts,
        bank_rate_last_updated: now_ts,
        avg_utilization: I80F48::ZERO,
        // 10% daily adjustment at 0% or 100% utilization
        adjustment_factor: I80F48::from_num(0.004),
        util0: I80F48::from_num(0.5),
        rate0: I80F48::from_num(0.018),
        util1: I80F48::from_num(0.75),
        rate1: I80F48::from_num(0.05),
        max_rate: I80F48::from_num(0.5),
        collected_fees_native: I80F48::ZERO,
        loan_origination_fee_rate: I80F48::from_num(0.0020),
        loan_fee_rate: I80F48::from_num(0.005),
        maint_asset_weight: I80F48::from_num(0),
        init_asset_weight: I80F48::from_num(0),
        maint_liab_weight: I80F48::from_num(1.4), // 2.5x
        init_liab_weight: I80F48::from_num(1.8),  // 1.25x
        liquidation_fee: I80F48::from_num(0.05),
        platform_liquidation_fee: I80F48::from_num(0.05),
        dust: I80F48::ZERO,
        token_index,
        bump: ctx.bumps.bank,
        mint_decimals: ctx.accounts.mint.decimals,
        bank_num: 0,
        min_vault_to_deposits_ratio: 0.2,
        net_borrow_limit_window_size_ts,
        last_net_borrows_window_start_ts: now_ts / net_borrow_limit_window_size_ts
            * net_borrow_limit_window_size_ts,
        net_borrow_limit_per_window_quote: 5_000_000_000, // $5k
        net_borrows_in_window: 0,
        borrow_weight_scale_start_quote: 5_000_000_000.0, // $5k
        deposit_weight_scale_start_quote: 5_000_000_000.0, // $5k
        reduce_only: 2,                                   // deposit-only
        force_close: 0,
        disable_asset_liquidation: 1,
        force_withdraw: 0,
        fees_withdrawn: 0,
        interest_target_utilization: 0.5,
        interest_curve_scaling: 4.0,
        maint_weight_shift_start: 0,
        maint_weight_shift_end: 0,
        maint_weight_shift_duration_inv: I80F48::ZERO,
        maint_weight_shift_asset_target: I80F48::ZERO,
        maint_weight_shift_liab_target: I80F48::ZERO,
        fallback_oracle: ctx.accounts.fallback_oracle.key(),
        deposit_limit: 0,
        zero_util_rate: I80F48::ZERO,
        collected_liquidation_fees: I80F48::ZERO,
        collected_collateral_fees: I80F48::ZERO,
        collateral_fee_per_day: 0.0, // TODO
        tier: fill_from_str("C")?,
        _padding1: Default::default(),
        _padding2: Default::default(),
        _padding3: Default::default(),
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
        token_index,
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
        seeds = [b"Bank".as_ref(), market.key().as_ref(), &token_index.to_le_bytes(), &FIRST_BANK_NUM.to_le_bytes()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<Bank>(),
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(
        init,
        seeds = [b"Vault".as_ref(), market.key().as_ref(), &token_index.to_le_bytes(), &FIRST_BANK_NUM.to_le_bytes()],
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
