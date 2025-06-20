use std::mem::size_of;

use anchor_lang::prelude::*;
use anchor_lang::{AnchorDeserialize, Discriminator};
// use fixed::traits::Fixed;
// use static_assertions::const_assert_eq;
// use std::mem::size_of;
use pyth_solana_receiver_sdk::price_update::VerificationLevel;
use bytemuck;
use static_assertions::const_assert_eq;
// use switchboard_on_demand::PullFeedAccountData;
// use switchboard_program::FastRoundResultAccountData;
// use switchboard_v2::AggregatorAccountData;

use crate::accounts_zerocopy::*;
use crate::error::PeachError;
use crate::custom_types::fixed_wrapper::FixedWrapper;
// use crate::error_msg;
// use crate::error::Contextable;
// use crate::state::stable_price::StablePriceModel;

use fixed::types::I80F48;
use derivative::Derivative;

// use crate::state::oracle_utils::{OracleAccountInfos, OraclePriceType}; // Removed this line

const DECIMAL_CONSTANT_ZERO_INDEX: i8 = 12;
const DECIMAL_CONSTANTS: [I80F48; 25] = [
    I80F48::from_bits((1 << 48) / 10i128.pow(12u32)),
    I80F48::from_bits((1 << 48) / 10i128.pow(11u32) + 1),
    I80F48::from_bits((1 << 48) / 10i128.pow(10u32)),
    I80F48::from_bits((1 << 48) / 10i128.pow(9u32) + 1),
    I80F48::from_bits((1 << 48) / 10i128.pow(8u32) + 1),
    I80F48::from_bits((1 << 48) / 10i128.pow(7u32) + 1),
    I80F48::from_bits((1 << 48) / 10i128.pow(6u32) + 1),
    I80F48::from_bits((1 << 48) / 10i128.pow(5u32)),
    I80F48::from_bits((1 << 48) / 10i128.pow(4u32)),
    I80F48::from_bits((1 << 48) / 10i128.pow(3u32) + 1), // 0.001
    I80F48::from_bits((1 << 48) / 10i128.pow(2u32) + 1), // 0.01
    I80F48::from_bits((1 << 48) / 10i128.pow(1u32) + 1), // 0.1
    I80F48::from_bits((1 << 48) * 10i128.pow(0u32)),     // 1, index 12
    I80F48::from_bits((1 << 48) * 10i128.pow(1u32)),     // 10
    I80F48::from_bits((1 << 48) * 10i128.pow(2u32)),     // 100
    I80F48::from_bits((1 << 48) * 10i128.pow(3u32)),     // 1000
    I80F48::from_bits((1 << 48) * 10i128.pow(4u32)),
    I80F48::from_bits((1 << 48) * 10i128.pow(5u32)),
    I80F48::from_bits((1 << 48) * 10i128.pow(6u32)),
    I80F48::from_bits((1 << 48) * 10i128.pow(7u32)),
    I80F48::from_bits((1 << 48) * 10i128.pow(8u32)),
    I80F48::from_bits((1 << 48) * 10i128.pow(9u32)),
    I80F48::from_bits((1 << 48) * 10i128.pow(10u32)),
    I80F48::from_bits((1 << 48) * 10i128.pow(11u32)),
    I80F48::from_bits((1 << 48) * 10i128.pow(12u32)),
];
pub const fn power_of_ten(decimals: i8) -> I80F48 {
    DECIMAL_CONSTANTS[(decimals + DECIMAL_CONSTANT_ZERO_INDEX) as usize]
}

pub const QUOTE_DECIMALS: i8 = 6;
pub const SOL_DECIMALS: i8 = 9;
pub const QUOTE_NATIVE_TO_UI: I80F48 = power_of_ten(-QUOTE_DECIMALS);

pub mod switchboard_v1_devnet_oracle {
    use anchor_lang::solana_program::declare_id;
    declare_id!("7azgmy1pFXHikv36q1zZASvFq5vFa39TT9NweVugKKTU");
}
pub mod switchboard_v2_mainnet_oracle {
    use anchor_lang::solana_program::declare_id;
    declare_id!("DtmE9D2CSB4L5D6A15mraeEjrGMm6auWVzgaD8hK2tZM");
}

pub mod switchboard_on_demand_devnet_oracle {
    use anchor_lang::solana_program::declare_id;
    declare_id!("SBondMDrcV3K4kxZR1HNVT7osZxAHVHgYXL5Ze1oMUv");
}
pub mod switchboard_on_demand_mainnet_oracle {
    use anchor_lang::solana_program::declare_id;
    declare_id!("SBondMDrcV3K4kxZR1HNVT7osZxAHVHgYXL5Ze1oMUv");
}

pub mod pyth_mainnet_usdc_oracle {
    use anchor_lang::solana_program::declare_id;
    declare_id!("Gnt27xtC473ZT2Mw5u8wZ68Z3gULkSTb5DuxJy7eJotD");
}

pub mod pyth_mainnet_sol_oracle {
    use anchor_lang::solana_program::declare_id;
    declare_id!("H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG");
}

pub mod usdc_mint_mainnet {
    use anchor_lang::solana_program::declare_id;
    declare_id!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
}

pub mod sol_mint_mainnet {
    use anchor_lang::solana_program::declare_id;
    declare_id!("So11111111111111111111111111111111111111112");
}

#[zero_copy]
#[derive(Derivative, PartialEq, Eq)]
#[derivative(Debug)]
pub struct OracleConfig {
    pub conf_filter: FixedWrapper,
    /// Max staleness for a price feed to be considered valid for trading, in slots.
    pub max_staleness_slots: i64,
    #[derivative(Debug = "ignore")]
    pub reserved: [u8; 72],
}

impl Default for OracleConfig {
    fn default() -> Self {
        OracleConfig {
            conf_filter: FixedWrapper::new(I80F48::from_num(1000.0)),
            max_staleness_slots: -1,
            reserved: [0; 72],
        }
    }
}

#[derive(AnchorDeserialize, AnchorSerialize, Debug, Default)]
pub struct OracleConfigParams {
    pub conf_filter: f32,
    pub max_staleness_slots: Option<u32>,
}

impl OracleConfigParams {
    pub fn to_oracle_config(&self) -> OracleConfig {
        OracleConfig {
            conf_filter: I80F48::from_num(self.conf_filter).into(),
            max_staleness_slots: self.max_staleness_slots.map(|v| v as i64).unwrap_or(-1),
            reserved: [0; 72],
        }
    }
}

#[derive(Clone, Copy, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub enum OracleType {
    Pyth,
    Stub,
    // SwitchboardV1, // Obsolete
    // SwitchboardV2,
    // OrcaCLMM,
    // RaydiumCLMM,
    // SwitchboardOnDemand,
    PythV2,
}

pub struct OracleState {
    pub price: I80F48,
    pub deviation: I80F48,
    pub last_update_slot: u64,
    pub last_update_time: Option<u64>,
    pub oracle_type: OracleType,
}

impl OracleState {
    #[inline]
    pub fn check_confidence_and_maybe_staleness(
        &self,
        config: &OracleConfig,
        now: Option<(u64, u64)>, // (now_ts, now_slot)
    ) -> Result<()> {
        if let Some((now_ts, now_slot)) = now {
            self.check_staleness(config, now_slot, now_ts)?;
        }
        self.check_confidence(config)
    }

    pub fn check_staleness(&self, config: &OracleConfig, now_slot: u64, now_ts: u64) -> Result<()> {
        if config.max_staleness_slots < 0 {
            return Ok(());
        }

        if self
            .last_update_slot
            .saturating_add(config.max_staleness_slots as u64)
            < now_slot
        {
            return Err(PeachError::OracleStale.into());
        }

        if self.last_update_time.is_some() {
            let current_time_in_msecs = now_ts * 1000;
            let last_update_time_in_msecs = self.last_update_time.unwrap() * 1000;
            let max_acceptable_update_age_in_ms = (config.max_staleness_slots as u64) * 450;

            let oldest_acceptable_time =
                current_time_in_msecs.saturating_sub(max_acceptable_update_age_in_ms);

            if last_update_time_in_msecs < oldest_acceptable_time {
                msg!("Oracle stale (using time fallback method: current time: {} vs published time: {})", current_time_in_msecs, last_update_time_in_msecs);
                return Err(PeachError::OracleStale.into());
            }
        }

        Ok(())
    }

    pub fn check_confidence(&self, config: &OracleConfig) -> Result<()> {
        if self.deviation > config.conf_filter.val() * self.price {
            return Err(PeachError::OracleConfidence.into());
        }
        Ok(())
    }
}

#[account(zero_copy)]
#[repr(C)]
#[derive(Default, Debug)]
pub struct StubOracle {
    // ABI: Clients rely on this being at offset 8
    pub market: Pubkey,
    // ABI: Clients rely on this being at offset 40
    pub mint: Pubkey,
    pub price: FixedWrapper,
    pub last_update_ts: i64,
    pub last_update_slot: u64,
    pub deviation: FixedWrapper,
    pub reserved: [u8; 16],
}
// const_assert_eq!(size_of::<StubOracle>(), 32 + 32 + 16 + 8 + 8 + 16 + 104);
// const_assert_eq!(size_of::<StubOracle>(), 216);
const_assert_eq!(size_of::<StubOracle>() % 16, 0);

pub fn check_is_valid_fallback_oracle(acc_info: &impl KeyedAccountReader) -> Result<()> {
    if acc_info.key() == &Pubkey::default() {
        return Ok(());
    };
    let oracle_type = determine_oracle_type(acc_info)?;
    let valid_oracle = match oracle_type {
        // OracleType::OrcaCLMM => {
        //     let whirlpool = load_orca_pool_state(acc_info)?;
        //     whirlpool.has_quote_token()
        // }
        // OracleType::RaydiumCLMM => {
        //     let pool = load_raydium_pool_state(acc_info)?;
        //     pool.has_quote_token()
        // }
        _ => true,
    };

    require!(valid_oracle, PeachError::UnexpectedOracle);
    Ok(())
}

pub fn determine_oracle_type(acc_info: &impl KeyedAccountReader) -> Result<OracleType> {
    let data = acc_info.data();

    if data.len()>= 4 && u32::from_le_bytes(data[0..4].try_into().unwrap()) == pyth_sdk_solana::state::MAGIC {
        return Ok(OracleType::Pyth);
    } else if data.len() >= 8 && &data[0..8] == StubOracle::DISCRIMINATOR {
        return Ok(OracleType::Stub);
    }
    // https://github.com/switchboard-xyz/switchboard-v2/blob/main/libraries/rs/src/aggregator.rs#L114
    // note: disc is not public, hence the copy pasta
    // else if data[0..8] == [217, 230, 65, 101, 201, 162, 27, 125] {
    //     return Ok(OracleType::SwitchboardV2);
    // }
    // note: this is the only known way of checking this
    // else if acc_info.owner() == &switchboard_v1_devnet_oracle::ID
    //     || acc_info.owner() == &switchboard_v2_mainnet_oracle::ID
    // {
    //     return Ok(OracleType::SwitchboardV1);
    // } else if acc_info.owner() == &switchboard_on_demand_devnet_oracle::ID
    //     || acc_info.owner() == &switchboard_on_demand_mainnet_oracle::ID
    // {
    //     return Ok(OracleType::SwitchboardOnDemand);
    // } else if acc_info.owner() == &orca_mainnet_whirlpool::ID {
    //     return Ok(OracleType::OrcaCLMM);
    // } else if acc_info.owner() == &raydium_mainnet::ID {
    //     return Ok(OracleType::RaydiumCLMM);
    // } 
    else if acc_info.owner() == &pyth_solana_receiver_sdk::ID {
        return Ok(OracleType::PythV2);
    }

    Err(PeachError::UnknownOracleType.into())
}

// Contains all oracle account infos that could be used to read price
pub struct OracleAccountInfos<'a, T: KeyedAccountReader> {
    pub oracle: &'a T,
    pub fallback_opt: Option<&'a T>,
    pub usdc_opt: Option<&'a T>,
    pub sol_opt: Option<&'a T>,
}

impl<'a, T: KeyedAccountReader> OracleAccountInfos<'a, T> {
    pub fn from_reader(acc_reader: &'a T) -> Self {
        OracleAccountInfos {
            oracle: acc_reader,
            fallback_opt: None,
            usdc_opt: None,
            sol_opt: None,
        }
    }
}

/// Returns the price of one native base token, in native quote tokens
///
/// Example: The price for SOL at 40 USDC/SOL it would return 0.04 (the unit is USDC-native/SOL-native)
///
/// This currently assumes that quote decimals (i.e. decimals for USD) is 6, like for USDC.
///
/// The staleness and confidence of the oracle is not checked. Use the functions on
/// OracleState to validate them if needed. That's why this function is called _unchecked.
pub fn oracle_state_unchecked<T: KeyedAccountReader>(
    acc_infos: &OracleAccountInfos<T>,
    base_decimals: u8,
) -> Result<OracleState> {
    oracle_state_unchecked_inner(acc_infos, base_decimals, false)
}

fn oracle_state_unchecked_inner<T: KeyedAccountReader>(
    acc_infos: &OracleAccountInfos<T>,
    base_decimals: u8,
    use_fallback: bool,
) -> Result<OracleState> {
    let acc_info = if use_fallback {
        acc_infos.fallback_opt.unwrap()
    } else {
        acc_infos.oracle
    };
    let oracle_type = determine_oracle_type(acc_info)?;
    match oracle_type {
        OracleType::Pyth => get_pyth_state(acc_info, base_decimals),
        OracleType::PythV2 => get_pyth_on_demand_state(acc_info, base_decimals),
        OracleType::Stub => {
            let data = acc_info.data();
            // The discriminator (8 bytes) is checked by determine_oracle_type.
            // The struct itself doesn't store the discriminator.
            let expected_data_len_without_discriminator = core::mem::size_of::<StubOracle>();
            if data.len() < 8 + expected_data_len_without_discriminator {
                // Consider a more specific error or logging
                return Err(ProgramError::AccountDataTooSmall.into());
            }
            let account_data_body = &data[8..8 + expected_data_len_without_discriminator];
            let stub_oracle: &StubOracle = bytemuck::from_bytes(account_data_body);

            Ok(OracleState {
                price: stub_oracle.price.val(),
                last_update_slot: stub_oracle.last_update_slot,
                deviation: stub_oracle.deviation.val(),
                last_update_time: Some(stub_oracle.last_update_ts as u64),
                oracle_type,
            })
        }
        // OracleType::SwitchboardV1 => {
        //     let swb_oracle = AggregatorAccountData::new(acc_info.as_ref())?;
        //     let ui_price = I80F48::from_num(swb_oracle.result.result.result);
        //     let ui_deviation = I80F48::from_num(swb_oracle.result.max_response - swb_oracle.result.min_response);
        //     let last_update_slot = swb_oracle.result.round_open_slot;
        //     let decimals = QUOTE_DECIMALS - (base_decimals as i8);
        //     let decimal_adj = power_of_ten(decimals);
        //     let price = ui_price * decimal_adj;
        //     let deviation = ui_deviation * decimal_adj;
        //     require_gte!(price, 0);
        //     OracleState {
        //         price,
        //         last_update_slot,
        //         deviation,
        //         oracle_type: OracleType::SwitchboardV1,
        //         last_update_time: None,
        //     }
        // }
        // OracleType::SwitchboardOnDemand => {
        //     let feed = bytemuck::from_bytes::<PullFeedAccountData>(acc_info.data())?;
        //     let ui_price: f64 = feed.value().ok_or_else(|| error_msg!("missing price"))?;
        //     let ui_deviation: f64 = feed.std_dev().ok_or_else(|| error_msg!("missing deviation"))?;
        //     let last_update_slot = feed.result.min_slot;
        //     let decimals = QUOTE_DECIMALS - (base_decimals as i8);
        //     let decimal_adj = power_of_ten(decimals);
        //     let price = I80F48::from_num(ui_price) * decimal_adj;
        //     let deviation = I80F48::from_num(ui_deviation) * decimal_adj;
        //     require_gte!(price, 0);
        //     OracleState {
        //         price,
        //         last_update_slot,
        //         deviation,
        //         oracle_type: OracleType::SwitchboardOnDemand,
        //         last_update_time: None,
        //     }
        // }
        // OracleType::OrcaCLMM => {
        //     let whirlpool = load_orca_pool_state(acc_info)?;
        //     let clmm_price = whirlpool.get_clmm_price();
        //     let quote_oracle_state = whirlpool.quote_state_unchecked(acc_infos)?;
        //     let price = clmm_price * quote_oracle_state.price;
        //     OracleState {
        //         price,
        //         last_update_slot: quote_oracle_state.last_update_slot,
        //         deviation: quote_oracle_state.deviation,
        //         oracle_type: OracleType::OrcaCLMM,
        //         last_update_time: None,
        //     }
        // }
        // OracleType::RaydiumCLMM => {
        //     let whirlpool = load_raydium_pool_state(acc_info)?;
        //     let clmm_price = whirlpool.get_clmm_price();
        //     let quote_oracle_state = whirlpool.quote_state_unchecked(acc_infos)?;
        //     let price = clmm_price * quote_oracle_state.price;
        //     OracleState {
        //         price,
        //         last_update_slot: quote_oracle_state.last_update_slot,
        //         deviation: quote_oracle_state.deviation,
        //         oracle_type: OracleType::RaydiumCLMM,
        //         last_update_time: None,
        //     }
        // }
    }
}

pub fn get_pyth_state(
    acc_info: &(impl KeyedAccountReader + ?Sized),
    base_decimals: u8,
) -> Result<OracleState> {
    let data = &acc_info.data();
    let price_account = pyth_sdk_solana::state::load_price_account(data).unwrap();
    let (price_data, last_update_slot) = pyth_get_price(acc_info.key(), price_account);

    let decimals = (price_account.expo as i8) + QUOTE_DECIMALS - (base_decimals as i8);
    let decimal_adj = power_of_ten(decimals);
    let price = I80F48::from_num(price_data.price) * decimal_adj;
    let deviation = I80F48::from_num(price_data.conf) * decimal_adj;
    require_gte!(price, 0);
    Ok(OracleState {
        price,
        last_update_slot,
        deviation,
        oracle_type: OracleType::Pyth,
        last_update_time: None,
    })
}

pub fn get_pyth_on_demand_state(
    acc_info: &(impl KeyedAccountReader + ?Sized),
    base_decimals: u8,
) -> Result<OracleState> {
    let mut data = acc_info.data();
    data = &data[8..];
    let price_account =
        pyth_solana_receiver_sdk::price_update::PriceUpdateV2::deserialize(&mut data).unwrap();
    if price_account.verification_level != VerificationLevel::Full {
        return Err(PeachError::OracleConfidence.into());
    }

    let decimals =
        (price_account.price_message.exponent as i8) + QUOTE_DECIMALS - (base_decimals as i8);
    let decimal_adj = power_of_ten(decimals);
    let price = I80F48::from_num(price_account.price_message.price) * decimal_adj;
    let deviation = I80F48::from_num(price_account.price_message.conf) * decimal_adj;
    let last_update_slot = price_account.posted_slot;

    let price_timestamp = price_account.price_message.publish_time;

    require_gte!(price, 0);
    Ok(OracleState {
        price,
        last_update_slot,
        deviation,
        oracle_type: OracleType::PythV2,
        last_update_time: Some(price_timestamp as u64),
    })
}

/// Get the pyth agg price if it's available, otherwise take the prev price.
///
/// Returns the publish slot in addition to the price info.
///
/// Also see pyth's PriceAccount::get_price_no_older_than().
fn pyth_get_price(
    _pubkey: &Pubkey,
    account: &pyth_sdk_solana::state::SolanaPriceAccount,
) -> (pyth_sdk_solana::Price, u64) {
    use pyth_sdk_solana::*;
    if account.agg.status == state::PriceStatus::Trading {
        (
            Price {
                conf: account.agg.conf,
                expo: account.expo,
                price: account.agg.price,
                publish_time: account.timestamp,
            },
            account.agg.pub_slot,
        )
    } else {
        (
            Price {
                conf: account.prev_conf,
                expo: account.expo,
                price: account.prev_price,
                publish_time: account.prev_timestamp,
            },
            account.prev_slot,
        )
    }
}

pub fn fallback_oracle_state_unchecked<T: KeyedAccountReader>(
    acc_infos: &OracleAccountInfos<T>,
    base_decimals: u8,
) -> Result<OracleState> {
    oracle_state_unchecked_inner(acc_infos, base_decimals, true)
}

pub fn oracle_log_context(
    name: &str,
    state: &OracleState,
    oracle_config: &OracleConfig,
    now: Option<(u64, u64)>,
) -> String {
    format!(
        "name: {}, price: {}, deviation: {}, last_update_slot: {}, now: {:?}, conf_filter: {:#?}",
        name,
        state.price.to_num::<f64>(),
        state.deviation.to_num::<f64>(),
        state.last_update_slot,
        now.unwrap_or_else(|| (u64::MAX, u64::MAX)),
        oracle_config.conf_filter.val().to_num::<f32>(),
    )
}

// const SCORE_MAX_AGE_SEC: u32 = 1800; // 30 minutes