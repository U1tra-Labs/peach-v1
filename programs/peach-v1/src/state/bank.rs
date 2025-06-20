use crate::custom_types::{F32Bytes, F64Bytes, TokenIndex};
use crate::error::*;
use crate::custom_types::fixed_wrapper::FixedWrapper;
use crate::state::*;
use crate::{accounts_zerocopy::KeyedAccountReader, custom_types::i80f48::ClampToInt};
use anchor_lang::prelude::*;
use derivative::Derivative;
use fixed::types::I80F48;

// use crate::util;

use super::{OracleAccountInfos, OracleConfig, StablePriceModel, TokenPosition};

pub const HOUR: i64 = 3600;
pub const YEAR_I80F48: I80F48 = I80F48::from_bits(31_536_000 * I80F48::ONE.to_bits());

#[derive(Derivative)]
// #[derivative(Debug)] // Commented out to test size mismatch
#[account(zero_copy)]
#[repr(C)]
pub struct Bank {
    // ABI: Clients rely on this being at offset 8
    pub market: Pubkey,

    #[derivative(Debug(format_with = "util::format_zero_terminated_utf8_bytes"))]
    pub name: [u8; 16],

    pub mint: Pubkey,
    pub vault: Pubkey,
    pub oracle: Pubkey,

    pub oracle_config: OracleConfig,
    pub stable_price_model: StablePriceModel,

    /// the index used to scale the value of an IndexedPosition
    /// TODO: should always be >= 0, add checks?
    pub deposit_index: FixedWrapper,
    pub borrow_index: FixedWrapper,

    /// deposits/borrows for this bank
    ///
    /// Note that these may become negative. It's perfectly fine for users to borrow on one bank
    /// (increasing indexed_borrows there) and paying back on another (possibly decreasing indexed_borrows
    /// below zero).
    ///
    /// The vault amount is not deducable from these values.
    ///
    /// These become meaningful when summed over all banks (like in update_index_and_rate).
    pub indexed_deposits: FixedWrapper,
    pub indexed_borrows: FixedWrapper,

    pub index_last_updated: u64,
    pub bank_rate_last_updated: u64,

    pub avg_utilization: FixedWrapper,

    pub adjustment_factor: FixedWrapper,

    /// The unscaled borrow interest curve is defined as continuous piecewise linear with the points:
    ///
    /// - 0% util: zero_util_rate
    /// - util0% util: rate0
    /// - util1% util: rate1
    /// - 100% util: max_rate
    ///
    /// The final rate is this unscaled curve multiplied by interest_curve_scaling.
    pub util0: FixedWrapper,
    pub rate0: FixedWrapper,
    pub util1: FixedWrapper,
    pub rate1: FixedWrapper,

    /// the 100% utilization rate
    ///
    /// This isn't the max_rate, since this still gets scaled by interest_curve_scaling,
    /// which is >=1.
    pub max_rate: FixedWrapper,

    /// Fees collected over the lifetime of the bank
    ///
    /// See fees_withdrawn for how much of the fees was withdrawn.
    /// See collected_liquidation_fees for the (included) subtotal for liquidation related fees.
    pub collected_fees_native: FixedWrapper,

    pub loan_origination_fee_rate: FixedWrapper,
    pub loan_fee_rate: FixedWrapper,

    // This is a _lot_ of bytes (64) - seems unnecessary
    // (could maybe store them in one byte each, as an informal U1F7?
    // that could store values between 0-2 and converting to I80F48 would be a cheap expand+shift)
    pub maint_asset_weight: FixedWrapper,
    pub init_asset_weight: FixedWrapper,
    pub maint_liab_weight: FixedWrapper,
    pub init_liab_weight: FixedWrapper,

    /// Liquidation fee that goes to the liqor.
    ///
    /// Liquidation always involves two tokens, and the sum of the two configured fees is used.
    ///
    /// A fraction of the price, like 0.05 for a 5% fee during liquidation.
    ///
    /// See also platform_liquidation_fee.
    pub liquidation_fee: FixedWrapper,

    // Collection of all fractions-of-native-tokens that got rounded away
    pub dust: FixedWrapper,

    // Index into TokenInfo on the market
    pub token_index: TokenIndex,

    pub bump: u8,

    pub mint_decimals: u8,

    pub bank_num: u32,

    /// The maximum utilization allowed when borrowing is 1-this value
    /// WARNING: Outdated name, kept for IDL compatibility
    pub min_vault_to_deposits_ratio: F64Bytes,

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
    pub borrow_weight_scale_start_quote: F64Bytes,

    /// Limit for collateral of deposits in native quote
    ///
    /// Once the deposits in the bank exceed this quote value, init_asset_weight is scaled
    /// down to keep the total collateral value constant.
    /// Set to f64::MAX to disable.
    ///
    /// See scaled_init_asset_weight().
    pub deposit_weight_scale_start_quote: F64Bytes,

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

    /// Target utilization: If actual utilization is higher, scale up interest.
    /// If it's lower, scale down interest (if possible)
    pub interest_target_utilization: F32Bytes,

    pub _padding1: [u8; 12],

    /// Current interest curve scaling, always >= 1.0
    ///
    /// Except when first migrating to having this field, then 0.0
    pub interest_curve_scaling: F64Bytes,

    /// Start timestamp in seconds at which maint weights should start to change away
    /// from maint_asset_weight, maint_liab_weight towards _asset_target and _liab_target.
    /// If _start and _end and _duration_inv are 0, no shift is configured.
    pub maint_weight_shift_start: u64,

    /// End timestamp in seconds until which the maint weights should reach the configured targets.
    pub maint_weight_shift_end: u64,

    /// zero means none, in token native
    pub deposit_limit: u64,

    /// Cache of the inverse of maint_weight_shift_end - maint_weight_shift_start,
    /// or zero if no shift is configured
    pub maint_weight_shift_duration_inv: FixedWrapper,
    /// Maint asset weight to reach at _shift_end.
    pub maint_weight_shift_asset_target: FixedWrapper,
    pub maint_weight_shift_liab_target: FixedWrapper,

    /// Oracle that may be used if the main oracle is unstale or not confident enough.
    /// If this is Pubkey::default(), no fallback is available.
    pub fallback_oracle: Pubkey,

    /// The unscaled borrow interest curve point for zero utilization.
    ///
    /// See util0, rate0, util1, rate1, max_rate
    pub zero_util_rate: FixedWrapper,

    /// Additional to liquidation_fee, but goes to the market owner instead of the liqor
    pub platform_liquidation_fee: FixedWrapper,

    /// Platform fees that were collected during liquidation (in native tokens)
    ///
    /// See also collected_fees_native and fees_withdrawn.
    pub collected_liquidation_fees: FixedWrapper,

    /// Collateral fees that have been collected (in native tokens)
    ///
    /// See also collected_fees_native and fees_withdrawn.
    pub collected_collateral_fees: FixedWrapper,

    /// The daily collateral fees rate for fully utilized collateral.
    pub collateral_fee_per_day: F32Bytes,

    #[derivative(Debug = "ignore")]
    pub reserved: [u8; 12],
}

pub struct WithdrawResult {
    pub position_is_active: bool,
    pub loan_origination_fee: I80F48,
    pub loan_amount: I80F48,
}

impl WithdrawResult {
    pub fn has_loan(&self) -> bool {
        self.loan_amount.is_positive()
    }
}

#[macro_export]
macro_rules! bank_seeds {
    ( $bank:expr ) => {
        &[
            b"Bank".as_ref(),
            $bank.market.as_ref(),
            $bank.token_index.0.to_le_bytes().as_ref(),
            $bank.bank_num.to_le_bytes().as_ref(),
            &[$bank.bump],
        ]
    };
}

pub use bank_seeds;

impl Bank {
    pub fn from_existing_bank(
        existing_bank: &Bank,
        vault: Pubkey,
        bank_num: u32,
        bump: u8,
    ) -> Self {
        Self {
            // values that must be reset/changed
            vault,
            indexed_deposits: FixedWrapper::zero(),
            indexed_borrows: FixedWrapper::zero(),
            collected_fees_native: FixedWrapper::zero(),
            collected_liquidation_fees: FixedWrapper::zero(),
            collected_collateral_fees: FixedWrapper::zero(),
            fees_withdrawn: 0,
            dust: FixedWrapper::zero(),
            net_borrows_in_window: 0,
            bump,
            bank_num,

            // values that can be copied
            // these are listed explicitly, so someone must make the decision when a
            // new field is added!
            name: existing_bank.name,
            market: existing_bank.market,
            mint: existing_bank.mint,
            oracle: existing_bank.oracle,
            deposit_index: existing_bank.deposit_index,
            borrow_index: existing_bank.borrow_index,
            index_last_updated: existing_bank.index_last_updated,
            bank_rate_last_updated: existing_bank.bank_rate_last_updated,
            avg_utilization: existing_bank.avg_utilization,
            adjustment_factor: existing_bank.adjustment_factor,
            util0: existing_bank.util0,
            rate0: existing_bank.rate0,
            util1: existing_bank.util1,
            rate1: existing_bank.rate1,
            max_rate: existing_bank.max_rate,
            loan_origination_fee_rate: existing_bank.loan_origination_fee_rate,
            loan_fee_rate: existing_bank.loan_fee_rate,
            maint_asset_weight: existing_bank.maint_asset_weight,
            init_asset_weight: existing_bank.init_asset_weight,
            maint_liab_weight: existing_bank.maint_liab_weight,
            init_liab_weight: existing_bank.init_liab_weight,
            liquidation_fee: existing_bank.liquidation_fee,
            token_index: existing_bank.token_index,
            mint_decimals: existing_bank.mint_decimals,
            oracle_config: existing_bank.oracle_config,
            stable_price_model: existing_bank.stable_price_model,
            min_vault_to_deposits_ratio: existing_bank.min_vault_to_deposits_ratio,
            net_borrow_limit_per_window_quote: existing_bank.net_borrow_limit_per_window_quote,
            net_borrow_limit_window_size_ts: existing_bank.net_borrow_limit_window_size_ts,
            last_net_borrows_window_start_ts: existing_bank.last_net_borrows_window_start_ts,
            borrow_weight_scale_start_quote: existing_bank.borrow_weight_scale_start_quote,
            deposit_weight_scale_start_quote: existing_bank.deposit_weight_scale_start_quote,
            reduce_only: existing_bank.reduce_only,
            force_close: existing_bank.force_close,
            disable_asset_liquidation: existing_bank.disable_asset_liquidation,
            force_withdraw: existing_bank.force_withdraw,
            tier: existing_bank.tier,
            interest_target_utilization: existing_bank.interest_target_utilization,
            interest_curve_scaling: existing_bank.interest_curve_scaling,
            maint_weight_shift_start: existing_bank.maint_weight_shift_start,
            maint_weight_shift_end: existing_bank.maint_weight_shift_end,
            maint_weight_shift_duration_inv: existing_bank.maint_weight_shift_duration_inv,
            maint_weight_shift_asset_target: existing_bank.maint_weight_shift_asset_target,
            maint_weight_shift_liab_target: existing_bank.maint_weight_shift_liab_target,
            fallback_oracle: existing_bank.fallback_oracle,
            deposit_limit: existing_bank.deposit_limit,
            zero_util_rate: existing_bank.zero_util_rate,
            platform_liquidation_fee: existing_bank.platform_liquidation_fee,
            collateral_fee_per_day: existing_bank.collateral_fee_per_day,
            _padding1: [0; 12],
            reserved: [0; 12],
        }
    }

    pub fn name(&self) -> &str {
        std::str::from_utf8(&self.name)
            .unwrap()
            .trim_matches(char::from(0))
    }

    // compute new avg utilization
    pub fn compute_new_avg_utilization(
        &self,
        indexed_total_deposits: I80F48,
        indexed_total_borrows: I80F48,
        now_ts: u64,
    ) -> I80F48 {
        if now_ts == 0 {
            return I80F48::ZERO;
        }

        let native_total_deposits = self.deposit_index.val() * indexed_total_deposits;
        let native_total_borrows = self.borrow_index.val() * indexed_total_borrows;
        let instantaneous_utilization =
            Self::instantaneous_utilization(native_total_deposits, native_total_borrows);

        // Compute a time-weighted average since bank_rate_last_updated.
        let previous_avg_time =
            I80F48::from_num(self.index_last_updated - self.bank_rate_last_updated);
        let diff_ts = I80F48::from_num(now_ts - self.index_last_updated);
        let new_avg_time = I80F48::from_num(now_ts - self.bank_rate_last_updated);
        if new_avg_time <= I80F48::ZERO {
            return instantaneous_utilization;
        }
        (self.avg_utilization.val() * previous_avg_time + instantaneous_utilization * diff_ts)
            / new_avg_time
    }

    // computes new optimal rates and max rate
    pub fn update_interest_rate_scaling(&mut self) {
        // Interest increases above target_util, decreases below
        let target_util = self.interest_target_utilization.val() as f64;

        // use avg_utilization and not instantaneous_utilization so that rates cannot be manipulated easily
        // also clamp to avoid unusually quick interest rate curve changes
        let avg_util = self.avg_utilization.val().to_num::<f64>().max(0.0).min(1.0);

        // move rates up when utilization is above optimal utilization, and vice versa
        // util factor is between -1 (avg util = 0) and +1 (avg util = 100%)
        let util_factor = if avg_util > target_util {
            (avg_util - target_util) / (1.0 - target_util)
        } else {
            (avg_util - target_util) / target_util
        };
        let adjustment = 1.0 + self.adjustment_factor.val().to_num::<f64>() * util_factor;

        self.interest_curve_scaling =
            F64Bytes::new((self.interest_curve_scaling.val() * adjustment).max(1.0));
    }

    pub fn compute_index(
        &self,
        indexed_total_deposits: I80F48,
        indexed_total_borrows: I80F48,
        diff_ts: I80F48,
    ) -> Result<(I80F48, I80F48, I80F48, I80F48, I80F48)> {
        // compute index based on utilization
        let native_total_deposits = self.deposit_index.val() * indexed_total_deposits;
        let native_total_borrows = self.borrow_index.val() * indexed_total_borrows;

        let instantaneous_utilization =
            Self::instantaneous_utilization(native_total_deposits, native_total_borrows);

        let borrow_rate = self.compute_interest_rate(instantaneous_utilization);

        // We want to grant depositors a rate that exactly matches the amount that is
        // taken from borrowers. That means:
        //   (new_deposit_index - old_deposit_index) * indexed_deposits
        //      = (new_borrow_index - old_borrow_index) * indexed_borrows
        // with
        //   new_deposit_index = old_deposit_index * (1 + deposit_rate) and
        //   new_borrow_index = old_borrow_index * (1 * borrow_rate)
        // we have
        //   deposit_rate = borrow_rate * (old_borrow_index * indexed_borrows) / (old_deposit_index * indexed_deposits)
        // and the latter factor is exactly instantaneous_utilization.
        let deposit_rate = borrow_rate * instantaneous_utilization;

        // The loan fee rate is not distributed to depositors.
        let borrow_rate_with_fees = borrow_rate + self.loan_fee_rate.val();
        let borrow_fees = native_total_borrows * self.loan_fee_rate.val() * diff_ts / YEAR_I80F48;

        let borrow_index_val = (self.borrow_index.val() * borrow_rate_with_fees * diff_ts)
            / YEAR_I80F48
            + self.borrow_index.val();
        let deposit_index_val = (self.deposit_index.val() * deposit_rate * diff_ts) / YEAR_I80F48
            + self.deposit_index.val();

        Ok((
            deposit_index_val,
            borrow_index_val,
            borrow_fees,
            borrow_rate,
            deposit_rate,
        ))
    }

    /// Current utilization, clamped to 0..1
    ///
    /// Above 100% utilization can happen natually when utilization is 100% and interest is paid out,
    /// increasing borrows more than deposits.
    fn instantaneous_utilization(
        native_total_deposits: I80F48,
        native_total_borrows: I80F48,
    ) -> I80F48 {
        if native_total_deposits == I80F48::ZERO {
            I80F48::ZERO
        } else {
            (native_total_borrows / native_total_deposits)
                .max(I80F48::ZERO)
                .min(I80F48::ONE)
        }
    }

    /// returns the current interest rate in APR
    #[inline(always)]
    pub fn compute_interest_rate(&self, utilization: I80F48) -> I80F48 {
        Bank::interest_rate_curve_calculator(
            utilization,
            self.zero_util_rate.val(),
            self.util0.val(),
            self.rate0.val(),
            self.util1.val(),
            self.rate1.val(),
            self.max_rate.val(),
            self.interest_curve_scaling.val(),
        )
    }

    /// calculator function that can be used to compute an interest
    /// rate based on the given parameters
    #[inline(always)]
    pub fn interest_rate_curve_calculator(
        utilization: I80F48,
        zero_util_rate: I80F48,
        util0: I80F48,
        rate0: I80F48,
        util1: I80F48,
        rate1: I80F48,
        max_rate: I80F48,
        scaling: f64,
    ) -> I80F48 {
        // Clamp to avoid negative or extremely high interest
        let utilization = utilization.max(I80F48::ZERO).min(I80F48::ONE);

        let v = if utilization <= util0 {
            let slope = (rate0 - zero_util_rate) / util0;
            zero_util_rate + slope * utilization
        } else if utilization <= util1 {
            let extra_util = utilization - util0;
            let slope = (rate1 - rate0) / (util1 - util0);
            rate0 + slope * extra_util
        } else {
            let extra_util = utilization - util1;
            let slope = (max_rate - rate1) / (I80F48::ONE - util1);
            rate1 + slope * extra_util
        };

        // scaling will be 0 when it's introduced
        if scaling == 0.0 {
            v
        } else {
            v * I80F48::from_num(scaling)
        }
    }

    pub fn verify(&self) -> Result<()> {
        require_gte!(self.oracle_config.conf_filter, FixedWrapper::zero());
        require_gte!(self.util0.val(), I80F48::ZERO);
        require_gte!(self.util1.val(), self.util0.val());
        require_gte!(I80F48::ONE, self.util1.val());
        require_gte!(self.rate0.val(), I80F48::ZERO);
        require_gte!(self.rate1.val(), I80F48::ZERO);
        require_gte!(self.max_rate.val(), I80F48::ZERO);
        require_gte!(self.adjustment_factor.val().to_num::<f64>(), 0.0_f64);
        require_gte!(self.loan_fee_rate.val().to_num::<f64>(), 0.0_f64);
        require_gte!(
            self.loan_origination_fee_rate.val().to_num::<f64>(),
            0.0_f64
        );
        require_gte!(self.stable_price_model.delay_growth_limit, 0.0);
        require_gte!(self.stable_price_model.stable_growth_limit, 0.0);
        require_gte!(self.init_asset_weight.val().to_num::<f64>(), 0.0_f64);
        require_gte!(self.maint_asset_weight.val(), self.init_asset_weight.val());
        require_gte!(self.maint_liab_weight.val().to_num::<f64>(), 0.0_f64);
        require_gte!(self.init_liab_weight.val(), self.maint_liab_weight.val());
        require_gte!(self.liquidation_fee.val().to_num::<f64>(), 0.0_f64);
        require_gte!(self.min_vault_to_deposits_ratio.val(), 0.0);
        require_gte!(1.0, self.min_vault_to_deposits_ratio.val());
        require_gte!(self.net_borrow_limit_per_window_quote, -1);
        require_gt!(self.borrow_weight_scale_start_quote.val(), 0.0);
        require_gt!(self.deposit_weight_scale_start_quote.val(), 0.0);
        require_gte!(2, self.reduce_only);
        require_gte!(self.interest_curve_scaling.val(), 1.0);
        require_gte!(self.interest_target_utilization.val(), 0.0);
        require_gte!(1.0, self.interest_target_utilization.val());
        require_gte!(
            self.maint_weight_shift_duration_inv.val().to_num::<f64>(),
            0.0_f64
        );
        require_gte!(
            self.maint_weight_shift_asset_target.val().to_num::<f64>(),
            0.0_f64
        );
        require_gte!(
            self.maint_weight_shift_liab_target.val().to_num::<f64>(),
            0.0_f64
        );
        require_gte!(self.zero_util_rate.val(), I80F48::ZERO);
        require_gte!(self.platform_liquidation_fee.val().to_num::<f64>(), 0.0_f64);
        if !self.allows_asset_liquidation() {
            require!(self.are_borrows_reduce_only(), PeachError::SomeError);
            require_eq!(self.maint_asset_weight.val(), I80F48::ZERO);
        }
        require_gte!(self.collateral_fee_per_day.val(), 0.0);
        if self.is_force_withdraw() {
            require!(self.are_deposits_reduce_only(), PeachError::SomeError);
            require!(!self.allows_asset_liquidation(), PeachError::SomeError);
            require_eq!(self.maint_asset_weight.val(), I80F48::ZERO);
        }
        Ok(())
    }

    pub fn are_borrows_reduce_only(&self) -> bool {
        self.reduce_only == 1 || self.reduce_only == 2
    }

    pub fn is_force_withdraw(&self) -> bool {
        self.force_withdraw == 1
    }

    pub fn are_deposits_reduce_only(&self) -> bool {
        self.reduce_only == 1
    }

    pub fn stable_price(&self) -> I80F48 {
        I80F48::from_num(self.stable_price_model.stable_price)
    }

    pub fn check_deposit_and_oo_limit(&self) -> Result<()> {
        if self.deposit_limit == 0 {
            return Ok(());
        }

        // Intentionally does not use remaining_deposits_until_limit(): That function
        // returns slightly less than the true limit to make sure depositing that amount
        // will not cause a limit overrun.
        let deposits = self.native_deposits();
        // let serum = I80F48::from(self.potential_serum_tokens);
        let total = deposits; // + serum;
        let remaining = I80F48::from_num(self.deposit_limit) - total;
        if remaining < I80F48::ZERO {
            return Err(error_msg_typed!(
                PeachError::BankDepositLimit,
                "deposit limit exceeded: remaining: {}, total: {}, limit: {}, deposits: {}", //, serum: {}",
                remaining,
                total,
                self.deposit_limit,
                deposits,
                // serum,
            ));
        }

        Ok(())
    }

    /// Returns the init asset weight, adjusted for the number of deposits on the bank.
    ///
    /// If max_collateral is 0, then the scaled init weight will be 0.
    /// Otherwise the weight is unadjusted until max_collateral and then scaled down
    /// such that scaled_init_weight * deposits remains constant.
    #[inline(always)]
    pub fn scaled_init_asset_weight(&self, price: I80F48) -> I80F48 {
        if self.deposit_weight_scale_start_quote.val() == f64::MAX {
            return self.init_asset_weight.val();
        }
        let all_deposits = self.native_deposits().to_num::<f64>(); // + self.potential_serum_tokens as f64;
        let deposits_quote = all_deposits * price.to_num::<f64>();
        if deposits_quote <= self.deposit_weight_scale_start_quote.val() {
            self.init_asset_weight.val()
        } else {
            // The next line is around 500 CU
            let scale = self.deposit_weight_scale_start_quote.val() / deposits_quote;
            self.init_asset_weight.val() * I80F48::from_num(scale)
        }
    }

    #[inline(always)]
    pub fn scaled_init_liab_weight(&self, price: I80F48) -> I80F48 {
        if self.borrow_weight_scale_start_quote.val() == f64::MAX {
            return self.init_liab_weight.val();
        }
        let borrows_quote = self.native_borrows().to_num::<f64>() * price.to_num::<f64>();
        if borrows_quote <= self.borrow_weight_scale_start_quote.val() {
            self.init_liab_weight.val()
        } else if self.borrow_weight_scale_start_quote.val() == 0.0 {
            // TODO: will certainly cause overflow, so it's not exactly what is needed; health should be -MAX?
            // maybe handling this case isn't super helpful?
            I80F48::MAX
        } else {
            // The next line is around 500 CU
            let scale = borrows_quote / self.borrow_weight_scale_start_quote.val();
            self.init_liab_weight.val() * I80F48::from_num(scale)
        }
    }

    /// Prevent borrowing away the full bank vault.
    /// Keep some in reserve to satisfy non-borrow withdraws.
    pub fn enforce_max_utilization_on_borrow(&self) -> Result<()> {
        self.enforce_max_utilization(
            I80F48::ONE - I80F48::from_num(self.min_vault_to_deposits_ratio.val()),
        )
    }

    /// Prevent borrowing away the full bank vault.
    /// Keep some in reserve to satisfy non-borrow withdraws.
    fn enforce_max_utilization(&self, max_utilization: I80F48) -> Result<()> {
        let bank_native_deposits = self.native_deposits();
        let bank_native_borrows = self.native_borrows();

        if bank_native_borrows > max_utilization * bank_native_deposits {
            return err!(PeachError::BankBorrowLimitReached).with_context(|| {
                format!(
                    "deposits {}, borrows {}, max utilization {}",
                    bank_native_deposits, bank_native_borrows, max_utilization,
                )
            });
        };

        Ok(())
    }

    pub fn remaining_net_borrows_quote(&self, oracle_price: I80F48) -> I80F48 {
        if self.net_borrows_in_window < 0 || self.net_borrow_limit_per_window_quote < 0 {
            return I80F48::MAX;
        }

        let price = oracle_price.max(self.stable_price());
        let net_borrows_quote = price
            .checked_mul_int(self.net_borrows_in_window.into())
            .unwrap();

        I80F48::from_num(self.net_borrow_limit_per_window_quote) - net_borrows_quote
    }

    pub fn check_net_borrows(&self, oracle_price: I80F48) -> Result<()> {
        let remaining_quote = self.remaining_net_borrows_quote(oracle_price);
        if remaining_quote < I80F48::ZERO {
            return Err(error_msg_typed!(PeachError::BankNetBorrowsLimitReached,
                    "net_borrows_in_window: {:?}, remaining quote: {:?}, net_borrow_limit_per_window_quote: {:?}, last_net_borrows_window_start_ts: {:?}",
                    self.net_borrows_in_window, remaining_quote, self.net_borrow_limit_per_window_quote, self.last_net_borrows_window_start_ts

            ));
        }

        Ok(())
    }

    pub fn enforce_borrows_lte_deposits(&self) -> Result<()> {
        self.enforce_max_utilization(I80F48::ONE)
    }

    pub fn allows_asset_liquidation(&self) -> bool {
        self.disable_asset_liquidation == 0
    }

    #[inline(always)]
    pub fn native_borrows(&self) -> I80F48 {
        self.borrow_index.val() * self.indexed_borrows.val()
    }

    #[inline(always)]
    pub fn native_deposits(&self) -> I80F48 {
        self.deposit_index.val() * self.indexed_deposits.val()
    }

    pub fn maint_weights(&self, now_ts: u64) -> (I80F48, I80F48) {
        if self.maint_weight_shift_duration_inv.is_zero() || now_ts <= self.maint_weight_shift_start
        {
            (self.maint_asset_weight.val(), self.maint_liab_weight.val())
        } else if now_ts >= self.maint_weight_shift_end {
            (
                self.maint_weight_shift_asset_target.val(),
                self.maint_weight_shift_liab_target.val(),
            )
        } else {
            let scale = I80F48::from_num(now_ts - self.maint_weight_shift_start)
                * self.maint_weight_shift_duration_inv.val();
            let asset = self.maint_asset_weight.val()
                + scale
                    * (self.maint_weight_shift_asset_target.val() - self.maint_asset_weight.val());
            let liab = self.maint_liab_weight.val()
                + scale
                    * (self.maint_weight_shift_liab_target.val() - self.maint_liab_weight.val());
            (asset, liab)
        }
    }

    /// Update the bank's net_borrows fields.
    ///
    /// If oracle_price is set, also do a net borrows check and error if the threshold is exceeded.
    pub fn update_net_borrows(&mut self, native_amount: I80F48, now_ts: u64) {
        let in_new_window =
            now_ts >= self.last_net_borrows_window_start_ts + self.net_borrow_limit_window_size_ts;

        let amount = native_amount.ceil().clamp_to_i64();

        self.net_borrows_in_window = if in_new_window {
            // reset to latest window
            self.last_net_borrows_window_start_ts = now_ts / self.net_borrow_limit_window_size_ts
                * self.net_borrow_limit_window_size_ts;
            amount
        } else {
            self.net_borrows_in_window + amount
        };
    }

    pub fn update_cumulative_interest(
        &self,
        position: &mut TokenPosition,
        opening_indexed_position: I80F48,
    ) {
        if opening_indexed_position.is_positive() {
            let interest = ((self.deposit_index.val() - position.previous_index.val())
                * opening_indexed_position)
                .to_num::<f64>();
            position.cumulative_deposit_interest += interest;
        } else {
            let interest = ((self.borrow_index.val() - position.previous_index.val())
                * opening_indexed_position)
                .to_num::<f64>();
            position.cumulative_borrow_interest -= interest;
        }

        if position.indexed_position.is_positive() {
            position.previous_index = self.deposit_index;
        } else {
            position.previous_index = self.borrow_index;
        }
    }

    /// Tries to return the primary oracle price, and if there is a confidence or staleness issue returns the fallback oracle price if possible.
    pub fn oracle_price<T: KeyedAccountReader>(
        &self,
        oracle_acc_infos: &OracleAccountInfos<T>,
        now: Option<(u64, u64)>, // (now_ts, now_slot)
    ) -> Result<I80F48> {
        require_keys_eq!(self.oracle, *oracle_acc_infos.oracle.key());
        let primary_state = oracle::oracle_state_unchecked(oracle_acc_infos, self.mint_decimals)?;
        let primary_ok =
            primary_state.check_confidence_and_maybe_staleness(&self.oracle_config, now);
        if primary_ok.is_oracle_error() && oracle_acc_infos.fallback_opt.is_some() {
            let fallback_oracle_acc = oracle_acc_infos.fallback_opt.unwrap();
            require_keys_eq!(self.fallback_oracle, *fallback_oracle_acc.key());
            let fallback_state =
                oracle::fallback_oracle_state_unchecked(&oracle_acc_infos, self.mint_decimals)?;
            let fallback_ok =
                fallback_state.check_confidence_and_maybe_staleness(&self.oracle_config, now);
            fallback_ok.with_context(|| {
                format!(
                    "{} {}",
                    oracle_log_context(self.name(), &primary_state, &self.oracle_config, now),
                    oracle_log_context(self.name(), &fallback_state, &self.oracle_config, now)
                )
            })?;
            Ok(fallback_state.price)
        } else {
            primary_ok.with_context(|| {
                oracle_log_context(self.name(), &primary_state, &self.oracle_config, now)
            })?;
            Ok(primary_state.price)
        }
    }

    /// Deposits `native_amount`.
    ///
    /// If the token position ends up positive but below one native token and this token
    /// position isn't marked as in-use, the token balance will be dusted, the position
    /// will be set to zero and this function returns Ok(false).
    ///
    /// native_amount must be >= 0
    /// fractional deposits can be relevant during liquidation, for example
    pub fn deposit(
        &mut self,
        position: &mut TokenPosition,
        native_amount: I80F48,
        now_ts: u64,
    ) -> Result<bool> {
        self.deposit_internal_wrapper(position, native_amount, !position.is_in_use(), now_ts)
    }

    pub fn deposit_internal_wrapper(
        &mut self,
        position: &mut TokenPosition,
        native_amount: I80F48,
        allow_dusting: bool,
        now_ts: u64,
    ) -> Result<bool> {
        let opening_indexed_position = position.indexed_position.val();
        let result = self.deposit_internal(position, native_amount, allow_dusting, now_ts)?;
        self.update_cumulative_interest(position, opening_indexed_position);
        Ok(result)
    }

    /// Internal function to deposit funds
    pub fn deposit_internal(
        &mut self,
        position: &mut TokenPosition,
        native_amount: I80F48,
        allow_dusting: bool,
        now_ts: u64,
    ) -> Result<bool> {
        require_gte!(native_amount, I80F48::ZERO);

        let native_position = position.native(self);

        // Adding DELTA to amount/index helps because (amount/index)*index <= amount, but
        // we want to ensure that users can withdraw the same amount they have deposited, so
        // (amount/index + delta)*index >= amount is a better guarantee.
        // Additionally, we require that we don't adjust values if
        // (native / index) * index == native, because we sometimes call this function with
        // values that are products of index.
        let div_rounding_up = |native: I80F48, index: I80F48| {
            let indexed = native / index;
            if (indexed * index) < native {
                indexed + I80F48::DELTA
            } else {
                indexed
            }
        };

        if native_position.is_negative() {
            // Only account for the borrows we are repaying
            self.update_net_borrows(native_position.max(-native_amount), now_ts);

            let new_native_position = native_position + native_amount;
            let indexed_change = div_rounding_up(native_amount, self.borrow_index.val());
            // this is only correct if it's not positive, because it scales the whole amount by borrow_index
            let new_indexed_value = position.indexed_position.val() + indexed_change;
            if new_indexed_value.is_negative() {
                // pay back borrows only, leaving a negative position
                self.indexed_borrows =
                    FixedWrapper::new(self.indexed_borrows.val() - indexed_change);
                position.indexed_position = FixedWrapper::new(new_indexed_value);
                return Ok(true);
            } else if new_native_position < I80F48::ONE && allow_dusting {
                // if there's less than one token deposited, zero the position
                self.dust = FixedWrapper::new(self.dust.val() + new_native_position);
                self.indexed_borrows =
                    FixedWrapper::new(self.indexed_borrows.val() + position.indexed_position.val());
                position.indexed_position = FixedWrapper::zero();
                return Ok(false);
            }

            // pay back all borrows
            self.indexed_borrows =
                FixedWrapper::new(self.indexed_borrows.val() + position.indexed_position.val()); // position.value is negative
            position.indexed_position = FixedWrapper::zero();
            // deposit the rest
            // note: .max(0) because there's a scenario where new_indexed_value == 0 and new_native_position < 0
            let _native_amount = new_native_position.max(I80F48::ZERO);
        }

        // add to deposits
        let indexed_change = div_rounding_up(native_amount, self.deposit_index.val());
        self.indexed_deposits = FixedWrapper::new(self.indexed_deposits.val() + indexed_change);
        position.indexed_position =
            FixedWrapper::new(position.indexed_position.val() + indexed_change);

        Ok(true)
    }

    /// Withdraws `native_amount` without applying the loan origination fee.
    ///
    /// If the token position ends up positive but below one native token and this token
    /// position isn't marked as in-use, the token balance will be dusted, the position
    /// will be set to zero and this function returns Ok(false).
    ///
    /// native_amount must be >= 0
    /// fractional withdraws can be relevant during liquidation, for example
    pub fn withdraw_without_fee(
        &mut self,
        position: &mut TokenPosition,
        native_amount: I80F48,
        now_ts: u64,
    ) -> Result<bool> {
        let position_is_active = self
            .withdraw_internal_wrapper(
                position,
                native_amount,
                false,
                !position.is_in_use(),
                now_ts,
            )?
            .position_is_active;

        Ok(position_is_active)
    }

    /// Withdraws `native_amount` while applying the loan origination fee if a borrow is created.
    ///
    /// If the token position ends up positive but below one native token and this token
    /// position isn't marked as in-use, the token balance will be dusted, the position
    /// will be set to zero and this function returns Ok(false).
    ///
    /// native_amount must be >= 0
    /// fractional withdraws can be relevant during liquidation, for example
    pub fn withdraw_with_fee(
        &mut self,
        position: &mut TokenPosition,
        native_amount: I80F48,
        now_ts: u64,
    ) -> Result<WithdrawResult> {
        self.withdraw_internal_wrapper(position, native_amount, true, !position.is_in_use(), now_ts)
    }

    /// Internal function to withdraw funds
    fn withdraw_internal_wrapper(
        &mut self,
        position: &mut TokenPosition,
        native_amount: I80F48,
        with_loan_origination_fee: bool,
        allow_dusting: bool,
        now_ts: u64,
    ) -> Result<WithdrawResult> {
        let opening_indexed_position = position.indexed_position.val();
        let res = self.withdraw_internal(
            position,
            native_amount,
            with_loan_origination_fee,
            allow_dusting,
            now_ts,
        );
        self.update_cumulative_interest(position, opening_indexed_position);
        res
    }

    /// Internal function to withdraw funds
    fn withdraw_internal(
        &mut self,
        position: &mut TokenPosition,
        mut native_amount: I80F48,
        with_loan_origination_fee: bool,
        allow_dusting: bool,
        now_ts: u64,
    ) -> Result<WithdrawResult> {
        require_gte!(native_amount, I80F48::ZERO);
        let native_position = position.native(self);

        if !native_position.is_negative() {
            let new_native_position = native_position - native_amount;
            if !new_native_position.is_negative() {
                // withdraw deposits only
                if new_native_position < I80F48::ONE && allow_dusting {
                    // zero the account collecting the leftovers in `dust`
                    self.dust = FixedWrapper::new(self.dust.val() + new_native_position);
                    self.indexed_deposits = FixedWrapper::new(
                        self.indexed_deposits.val() - position.indexed_position.val(),
                    );
                    position.indexed_position = FixedWrapper::zero();
                    return Ok(WithdrawResult {
                        position_is_active: false,
                        loan_origination_fee: I80F48::ZERO,
                        loan_amount: I80F48::ZERO,
                    });
                } else {
                    // withdraw some deposits leaving a positive balance
                    let indexed_change = native_amount / self.deposit_index.val();
                    self.indexed_deposits =
                        FixedWrapper::new(self.indexed_deposits.val() - indexed_change);
                    position.indexed_position =
                        FixedWrapper::new(position.indexed_position.val() - indexed_change);
                    return Ok(WithdrawResult {
                        position_is_active: true,
                        loan_origination_fee: I80F48::ZERO,
                        loan_amount: I80F48::ZERO,
                    });
                }
            }

            // withdraw all deposits
            self.indexed_deposits =
                FixedWrapper::new(self.indexed_deposits.val() - position.indexed_position.val());
            position.indexed_position = FixedWrapper::zero();
            // borrow the rest
            native_amount = -new_native_position;
        }

        let mut loan_origination_fee = I80F48::ZERO;
        if with_loan_origination_fee {
            loan_origination_fee = self.loan_origination_fee_rate.val() * native_amount;
            self.collected_fees_native =
                FixedWrapper::new(self.collected_fees_native.val() + loan_origination_fee);
            native_amount += loan_origination_fee;
        }

        // add to borrows
        let indexed_change = native_amount / self.borrow_index.val();
        self.indexed_borrows = FixedWrapper::new(self.indexed_borrows.val() + indexed_change);
        position.indexed_position =
            FixedWrapper::new(position.indexed_position.val() - indexed_change);

        // net borrows requires updating in only this case, since other branches of the method deal with
        // withdraws and not borrows
        self.update_net_borrows(native_amount, now_ts);

        Ok(WithdrawResult {
            position_is_active: true,
            loan_origination_fee,
            loan_amount: native_amount,
        })
    }
}
