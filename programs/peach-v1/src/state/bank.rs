use anchor_lang::prelude::*;
use derivative::Derivative;
use fixed::types::I80F48;
use crate::state::*;
use crate::error::*;
use crate::{accounts_zerocopy::KeyedAccountReader, i80f48::ClampToInt};

use crate::util;

use super::{OracleAccountInfos, OracleConfig, StablePriceModel, TokenPosition};

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
    /// Note that these may become negative. It's perfectly fine for users to borrow on one bank
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

    // Index into TokenInfo on the market
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

    /// Oracle that may be used if the main oracle is unstale or not confident enough.
    /// If this is Pubkey::default(), no fallback is available.
    pub fallback_oracle: Pubkey,

    /// zero means none, in token native
    pub deposit_limit: u64,

    /// The unscaled borrow interest curve point for zero utilization.
    ///
    /// See util0, rate0, util1, rate1, max_rate
    pub zero_util_rate: I80F48,

    /// Additional to liquidation_fee, but goes to the market owner instead of the liqor
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

    pub fn verify(&self) -> Result<()> {
        require_gte!(self.oracle_config.conf_filter, 0.0);
        require_gte!(self.util0, I80F48::ZERO);
        require_gte!(self.util1, self.util0);
        require_gte!(I80F48::ONE, self.util1);
        require_gte!(self.rate0, I80F48::ZERO);
        require_gte!(self.rate1, I80F48::ZERO);
        require_gte!(self.max_rate, I80F48::ZERO);
        require_gte!(self.adjustment_factor, 0.0);
        require_gte!(self.loan_fee_rate, 0.0);
        require_gte!(self.loan_origination_fee_rate, 0.0);
        require_gte!(self.stable_price_model.delay_growth_limit, 0.0);
        require_gte!(self.stable_price_model.stable_growth_limit, 0.0);
        require_gte!(self.init_asset_weight, 0.0);
        require_gte!(self.maint_asset_weight, self.init_asset_weight);
        require_gte!(self.maint_liab_weight, 0.0);
        require_gte!(self.init_liab_weight, self.maint_liab_weight);
        require_gte!(self.liquidation_fee, 0.0);
        require_gte!(self.min_vault_to_deposits_ratio, 0.0);
        require_gte!(1.0, self.min_vault_to_deposits_ratio);
        require_gte!(self.net_borrow_limit_per_window_quote, -1);
        require_gt!(self.borrow_weight_scale_start_quote, 0.0);
        require_gt!(self.deposit_weight_scale_start_quote, 0.0);
        require_gte!(2, self.reduce_only);
        require_gte!(self.interest_curve_scaling, 1.0);
        require_gte!(self.interest_target_utilization, 0.0);
        require_gte!(1.0, self.interest_target_utilization);
        require_gte!(self.maint_weight_shift_duration_inv, 0.0);
        require_gte!(self.maint_weight_shift_asset_target, 0.0);
        require_gte!(self.maint_weight_shift_liab_target, 0.0);
        require_gte!(self.zero_util_rate, I80F48::ZERO);
        require_gte!(self.platform_liquidation_fee, 0.0);
        if !self.allows_asset_liquidation() {
            require!(self.are_borrows_reduce_only(), PeachError::SomeError);
            require_eq!(self.maint_asset_weight, I80F48::ZERO);
        }
        require_gte!(self.collateral_fee_per_day, 0.0);
        if self.is_force_withdraw() {
            require!(self.are_deposits_reduce_only(), PeachError::SomeError);
            require!(!self.allows_asset_liquidation(), PeachError::SomeError);
            require_eq!(self.maint_asset_weight, I80F48::ZERO);
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
        let remaining = I80F48::from(self.deposit_limit) - total;
        if remaining < 0 {
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
        if self.deposit_weight_scale_start_quote == f64::MAX {
            return self.init_asset_weight;
        }
        let all_deposits =
            self.native_deposits().to_num::<f64>(); // + self.potential_serum_tokens as f64;
        let deposits_quote = all_deposits * price.to_num::<f64>();
        if deposits_quote <= self.deposit_weight_scale_start_quote {
            self.init_asset_weight
        } else {
            // The next line is around 500 CU
            let scale = self.deposit_weight_scale_start_quote / deposits_quote;
            self.init_asset_weight * I80F48::from_num(scale)
        }
    }

    #[inline(always)]
    pub fn scaled_init_liab_weight(&self, price: I80F48) -> I80F48 {
        if self.borrow_weight_scale_start_quote == f64::MAX {
            return self.init_liab_weight;
        }
        let borrows_quote = self.native_borrows().to_num::<f64>() * price.to_num::<f64>();
        if borrows_quote <= self.borrow_weight_scale_start_quote {
            self.init_liab_weight
        } else if self.borrow_weight_scale_start_quote == 0.0 {
            // TODO: will certainly cause overflow, so it's not exactly what is needed; health should be -MAX?
            // maybe handling this case isn't super helpful?
            I80F48::MAX
        } else {
            // The next line is around 500 CU
            let scale = borrows_quote / self.borrow_weight_scale_start_quote;
            self.init_liab_weight * I80F48::from_num(scale)
        }
    }

    /// Prevent borrowing away the full bank vault.
    /// Keep some in reserve to satisfy non-borrow withdraws.
    pub fn enforce_max_utilization_on_borrow(&self) -> Result<()> {
        self.enforce_max_utilization(
            I80F48::ONE - I80F48::from_num(self.min_vault_to_deposits_ratio),
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

        I80F48::from(self.net_borrow_limit_per_window_quote) - net_borrows_quote
    }

    pub fn check_net_borrows(&self, oracle_price: I80F48) -> Result<()> {
        let remaining_quote = self.remaining_net_borrows_quote(oracle_price);
        if remaining_quote < 0 {
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
        self.borrow_index * self.indexed_borrows
    }

    #[inline(always)]
    pub fn native_deposits(&self) -> I80F48 {
        self.deposit_index * self.indexed_deposits
    }

    pub fn maint_weights(&self, now_ts: u64) -> (I80F48, I80F48) {
        if self.maint_weight_shift_duration_inv.is_zero() || now_ts <= self.maint_weight_shift_start
        {
            (self.maint_asset_weight, self.maint_liab_weight)
        } else if now_ts >= self.maint_weight_shift_end {
            (
                self.maint_weight_shift_asset_target,
                self.maint_weight_shift_liab_target,
            )
        } else {
            let scale = I80F48::from(now_ts - self.maint_weight_shift_start)
                * self.maint_weight_shift_duration_inv;
            let asset = self.maint_asset_weight
                + scale * (self.maint_weight_shift_asset_target - self.maint_asset_weight);
            let liab = self.maint_liab_weight
                + scale * (self.maint_weight_shift_liab_target - self.maint_liab_weight);
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
            let interest = ((self.deposit_index - position.previous_index)
                * opening_indexed_position)
                .to_num::<f64>();
            position.cumulative_deposit_interest += interest;
        } else {
            let interest = ((self.borrow_index - position.previous_index)
                * opening_indexed_position)
                .to_num::<f64>();
            position.cumulative_borrow_interest -= interest;
        }

        if position.indexed_position.is_positive() {
            position.previous_index = self.deposit_index
        } else {
            position.previous_index = self.borrow_index
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
        let opening_indexed_position = position.indexed_position;
        let result = self.deposit_internal(position, native_amount, allow_dusting, now_ts)?;
        self.update_cumulative_interest(position, opening_indexed_position);
        Ok(result)
    }

    /// Internal function to deposit funds
    pub fn deposit_internal(
        &mut self,
        position: &mut TokenPosition,
        mut native_amount: I80F48,
        allow_dusting: bool,
        now_ts: u64,
    ) -> Result<bool> {
        require_gte!(native_amount, 0);

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
            let indexed_change = div_rounding_up(native_amount, self.borrow_index);
            // this is only correct if it's not positive, because it scales the whole amount by borrow_index
            let new_indexed_value = position.indexed_position + indexed_change;
            if new_indexed_value.is_negative() {
                // pay back borrows only, leaving a negative position
                self.indexed_borrows -= indexed_change;
                position.indexed_position = new_indexed_value;
                return Ok(true);
            } else if new_native_position < I80F48::ONE && allow_dusting {
                // if there's less than one token deposited, zero the position
                self.dust += new_native_position;
                self.indexed_borrows += position.indexed_position;
                position.indexed_position = I80F48::ZERO;
                return Ok(false);
            }

            // pay back all borrows
            self.indexed_borrows += position.indexed_position; // position.value is negative
            position.indexed_position = I80F48::ZERO;
            // deposit the rest
            // note: .max(0) because there's a scenario where new_indexed_value == 0 and new_native_position < 0
            native_amount = new_native_position.max(I80F48::ZERO);
        }

        // add to deposits
        let indexed_change = div_rounding_up(native_amount, self.deposit_index);
        self.indexed_deposits += indexed_change;
        position.indexed_position += indexed_change;

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
        let opening_indexed_position = position.indexed_position;
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
        require_gte!(native_amount, 0);
        let native_position = position.native(self);

        if !native_position.is_negative() {
            let new_native_position = native_position - native_amount;
            if !new_native_position.is_negative() {
                // withdraw deposits only
                if new_native_position < I80F48::ONE && allow_dusting {
                    // zero the account collecting the leftovers in `dust`
                    self.dust += new_native_position;
                    self.indexed_deposits -= position.indexed_position;
                    position.indexed_position = I80F48::ZERO;
                    return Ok(WithdrawResult {
                        position_is_active: false,
                        loan_origination_fee: I80F48::ZERO,
                        loan_amount: I80F48::ZERO,
                    });
                } else {
                    // withdraw some deposits leaving a positive balance
                    let indexed_change = native_amount / self.deposit_index;
                    self.indexed_deposits -= indexed_change;
                    position.indexed_position -= indexed_change;
                    return Ok(WithdrawResult {
                        position_is_active: true,
                        loan_origination_fee: I80F48::ZERO,
                        loan_amount: I80F48::ZERO,
                    });
                }
            }

            // withdraw all deposits
            self.indexed_deposits -= position.indexed_position;
            position.indexed_position = I80F48::ZERO;
            // borrow the rest
            native_amount = -new_native_position;
        }

        let mut loan_origination_fee = I80F48::ZERO;
        if with_loan_origination_fee {
            loan_origination_fee = self.loan_origination_fee_rate * native_amount;
            self.collected_fees_native += loan_origination_fee;
            native_amount += loan_origination_fee;
        }

        // add to borrows
        let indexed_change = native_amount / self.borrow_index;
        self.indexed_borrows += indexed_change;
        position.indexed_position -= indexed_change;

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