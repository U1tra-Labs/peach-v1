use anchor_lang::prelude::*;
use derivative::Derivative;
use fixed::types::I80F48;

use crate::util;

use super::{OracleConfig, StablePriceModel};

pub type TokenIndex = u16;

#[derive(Derivative)]
#[derivative(Debug)]
#[account(zero_copy)]
pub struct Bank{

    // ABI: Clients rely on this being at offset 8
    pub market: Pubkey,

    #[derivative(Debug(format_with = "util::format_zero_terminated_utf8_bytes"))]
    pub name: [u8; 16],

    pub _padding1: [u8; 16],

    pub mint: Pubkey,
    pub vault: Pubkey,
    pub oracle: Pubkey,

    pub oracle_config: OracleConfig,
    pub stable_price_model: StablePriceModel,

    /// the index used to scale the value of an IndexedPosition
    /// TODO: should always be >= 0, add checks?
    pub deposit_index: I80F48,
    pub borrow_index: I80F48,

    /// deposits/borrows for this bank
    ///
    /// Note that these may become negative. It's perfectly fine for users to borrow one one bank
    /// (increasing indexed_borrows there) and paying back on another (possibly decreasing indexed_borrows
    /// below zero).
    ///
    /// The vault amount is not deducable from these values.
    ///
    /// These become meaningful when summed over all banks (like in update_index_and_rate).
    pub indexed_deposits: I80F48,
    pub indexed_borrows: I80F48,

    pub index_last_updated: u64,
    pub bank_rate_last_updated: u64,

    pub avg_utilization: I80F48,

    pub adjustment_factor: I80F48,

    /// The unscaled borrow interest curve is defined as continuous piecewise linear with the points:
    ///
    /// - 0% util: zero_util_rate
    /// - util0% util: rate0
    /// - util1% util: rate1
    /// - 100% util: max_rate
    ///
    /// The final rate is this unscaled curve multiplied by interest_curve_scaling.
    pub util0: I80F48,
    pub rate0: I80F48,
    pub util1: I80F48,
    pub rate1: I80F48,

    /// the 100% utilization rate
    ///
    /// This isn't the max_rate, since this still gets scaled by interest_curve_scaling,
    /// which is >=1.
    pub max_rate: I80F48,

    /// Fees collected over the lifetime of the bank
    ///
    /// See fees_withdrawn for how much of the fees was withdrawn.
    /// See collected_liquidation_fees for the (included) subtotal for liquidation related fees.
    pub collected_fees_native: I80F48,

    pub loan_origination_fee_rate: I80F48,
    pub loan_fee_rate: I80F48,

    // This is a _lot_ of bytes (64) - seems unnecessary
    // (could maybe store them in one byte each, as an informal U1F7?
    // that could store values between 0-2 and converting to I80F48 would be a cheap expand+shift)
    pub maint_asset_weight: I80F48,
    pub init_asset_weight: I80F48,
    pub maint_liab_weight: I80F48,
    pub init_liab_weight: I80F48,

    /// Liquidation fee that goes to the liqor.
    ///
    /// Liquidation always involves two tokens, and the sum of the two configured fees is used.
    ///
    /// A fraction of the price, like 0.05 for a 5% fee during liquidation.
    ///
    /// See also platform_liquidation_fee.
    pub liquidation_fee: I80F48,

    // Collection of all fractions-of-native-tokens that got rounded away
    pub dust: I80F48,

    // Index into TokenInfo on the group
    pub token_index: TokenIndex,

    pub bump: u8,

    pub mint_decimals: u8,

    pub bank_num: u32,

    /// The maximum utilization allowed when borrowing is 1-this value
    /// WARNING: Outdated name, kept for IDL compatibility
    pub min_vault_to_deposits_ratio: f64,

    /// Size in seconds of a net borrows window
    pub net_borrow_limit_window_size_ts: u64,
    /// Timestamp at which the last net borrows window started
    pub last_net_borrows_window_start_ts: u64,
    /// Net borrow limit per window in quote native; set to -1 to disable.
    pub net_borrow_limit_per_window_quote: i64,
    /// Sum of all deposits and borrows in the last window, in native units.
    pub net_borrows_in_window: i64,

    /// Soft borrow limit in native quote
    ///
    /// Once the borrows on the bank exceed this quote value, init_liab_weight is scaled up.
    /// Set to f64::MAX to disable.
    ///
    /// See scaled_init_liab_weight().
    pub borrow_weight_scale_start_quote: f64,

    /// Limit for collateral of deposits in native quote
    ///
    /// Once the deposits in the bank exceed this quote value, init_asset_weight is scaled
    /// down to keep the total collateral value constant.
    /// Set to f64::MAX to disable.
    ///
    /// See scaled_init_asset_weight().
    pub deposit_weight_scale_start_quote: f64,

    // We have 3 modes
    // 0 - Off,
    // 1 - ReduceDepositsReduceBorrows - standard
    // 2 - ReduceBorrows - borrows can only be reduced, but deposits have no restriction, special case for
    //                 force close mode, where liqor should first acquire deposits before closing liqee's borrows
    pub reduce_only: u8,
    pub force_close: u8,

    /// If set to 1, deposits cannot be liquidated when an account is liquidatable.
    /// That means bankrupt accounts may still have assets of this type deposited.
    pub disable_asset_liquidation: u8,

    pub force_withdraw: u8,

    #[derivative(Debug(format_with = "util::format_zero_terminated_utf8_bytes"))]
    pub tier: [u8; 4],

    // Do separate bookkeping for how many tokens were withdrawn
    // This ensures that collected_fees_native is strictly increasing for stats gathering purposes
    pub fees_withdrawn: u64,

    // pub padding1: [u8; 12],

    /// Target utilization: If actual utilization is higher, scale up interest.
    /// If it's lower, scale down interest (if possible)
    pub interest_target_utilization: f32,

    pub _padding2: [u8; 4],

    /// Current interest curve scaling, always >= 1.0
    ///
    /// Except when first migrating to having this field, then 0.0
    pub interest_curve_scaling: f64,

    /// Start timestamp in seconds at which maint weights should start to change away
    /// from maint_asset_weight, maint_liab_weight towards _asset_target and _liab_target.
    /// If _start and _end and _duration_inv are 0, no shift is configured.
    pub maint_weight_shift_start: u64,
    /// End timestamp in seconds until which the maint weights should reach the configured targets.
    pub maint_weight_shift_end: u64,
    /// Cache of the inverse of maint_weight_shift_end - maint_weight_shift_start,
    /// or zero if no shift is configured
    pub maint_weight_shift_duration_inv: I80F48,
    /// Maint asset weight to reach at _shift_end.
    pub maint_weight_shift_asset_target: I80F48,
    pub maint_weight_shift_liab_target: I80F48,

    /// Oracle that may be used if the main oracle is stale or not confident enough.
    /// If this is Pubkey::default(), no fallback is available.
    pub fallback_oracle: Pubkey,

    /// zero means none, in token native
    pub deposit_limit: u64,

    /// The unscaled borrow interest curve point for zero utilization.
    ///
    /// See util0, rate0, util1, rate1, max_rate
    pub zero_util_rate: I80F48,

    /// Additional to liquidation_fee, but goes to the group owner instead of the liqor
    pub platform_liquidation_fee: I80F48,

    /// Platform fees that were collected during liquidation (in native tokens)
    ///
    /// See also collected_fees_native and fees_withdrawn.
    pub collected_liquidation_fees: I80F48,

    /// Collateral fees that have been collected (in native tokens)
    ///
    /// See also collected_fees_native and fees_withdrawn.
    pub collected_collateral_fees: I80F48,

    /// The daily collateral fees rate for fully utilized collateral.
    pub collateral_fee_per_day: f32,

    pub _padding3: [u8; 4]

    // #[derivative(Debug = "ignore")]
    // pub reserved: [u8; 1900],
}

#[macro_export]
macro_rules! bank_seeds {
    ( $bank:expr ) => {
        &[
            b"Bank".as_ref(),
            $bank.market.as_ref(),
            $bank.token_index.to_le_bytes(),
            &bank.bank_num.to_le_bytes(),
            &[$bank.bump],
        ]
    };
}

pub use bank_seeds;

impl Bank{
    pub fn name(&self) -> &str {
        std::str::from_utf8(&self.name)
            .unwrap()
            .trim_matches(char::from(0))
    }
}