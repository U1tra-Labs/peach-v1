use anchor_lang::prelude::*;
use derivative::Derivative;

/// Maintains a "stable_price" based on the oracle price.
///
/// The stable price follows the oracle price, but its relative rate of
/// change is limited (to `stable_growth_limit`) and futher reduced if
/// the oracle price is far from the `delay_price`.
///
/// Conceptually the `delay_price` is itself a time delayed
/// (`24 * delay_interval_seconds`, assume 24h) and relative rate of change limited
/// function of the oracle price. It is implemented as averaging the oracle
/// price over every `delay_interval_seconds` (assume 1h) and then applying the
/// `delay_growth_limit` between intervals.
#[zero_copy]
#[derive(Derivative)]
#[derivative(Debug)]
pub struct StablePriceModel {
    /// Current stable price to use in health
    pub stable_price: f64,

    pub last_update_timestamp: u64,

    /// Stored delay_price for each delay_interval.
    /// If we want the delay_price to be 24h delayed, we would store one for each hour.
    /// This is used in a cyclical way: We use the maximally-delayed value at delay_interval_index
    /// and once enough time passes to move to the next delay interval, that gets overwritten and
    /// we use the next one.
    pub delay_prices: [f64; 24],

    /// The delay price is based on an average over each delay_interval. The contributions
    /// to the average are summed up here.
    pub delay_accumulator_price: f64,

    /// Accumulating the total time for the above average.
    pub delay_accumulator_time: u32,

    /// Length of a delay_interval
    pub delay_interval_seconds: u32,

    /// Maximal relative difference between two delay_price in consecutive intervals.
    pub delay_growth_limit: f32,

    /// Maximal per-second relative difference of the stable price.
    /// It gets further reduced if stable and delay price disagree.
    pub stable_growth_limit: f32,

    /// The delay_interval_index that update() was last called on.
    pub last_delay_interval_index: u8,

    /// If set to 1, the stable price will reset on the next non-zero price it sees.
    pub reset_on_nonzero_price: u8,

    #[derivative(Debug = "ignore")]
    pub padding: [u8; 6],

    #[derivative(Debug = "ignore")]
    pub reserved: [u8; 48],
}

impl Default for StablePriceModel {
    fn default() -> Self {
        Self {
            stable_price: 0.0,
            last_update_timestamp: 0,
            delay_prices: [0.0; 24],
            delay_accumulator_price: 0.0,
            delay_accumulator_time: 0,
            delay_interval_seconds: 60 * 60, // 1h, for a total delay of 24h
            delay_growth_limit: 0.06,        // 6% per hour, 400% per day
            stable_growth_limit: 0.0003, // 0.03% per second, 293% in 1h if updated every 10s, 281% in 1h if updated every 5min
            last_delay_interval_index: 0,
            reset_on_nonzero_price: 0,
            padding: Default::default(),
            reserved: [0; 48],
        }
    }
}

impl StablePriceModel {
        pub fn reset_to_price(&mut self, oracle_price: f64, now_ts: u64) {
        self.stable_price = oracle_price;
        self.delay_prices = [oracle_price; 24];
        self.delay_accumulator_price = 0.0;
        self.delay_accumulator_time = 0;
        self.last_update_timestamp = now_ts;
        self.reset_on_nonzero_price = if oracle_price > 0.0 { 0 } else { 1 };
    }
}