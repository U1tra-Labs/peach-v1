/*!
 * This module deals with computing different types of health for a peach account.
 *
 * Health is a number in USD and represents a risk-engine assessment of the account's
 * positions and open orders. The larger the health the better. Negative health
 * often means some action is necessary or a limitation is placed on the user.
 *
 * The different types of health are described in the HealthType enum.
 *
 * The key struct in this module is HealthCache, typically constructed by the
 * new_health_cache() function. With it, the different health types can be
 * computed.
 *
 * The HealthCache holds the data it needs in TokenInfo, Serum3Info and PerpInfo.
 */
use anchor_lang::prelude::*;
use fixed::types::I80F48;

use crate::custom_types::fixed_wrapper::FixedWrapper;
use crate::custom_types::TokenIndex;
use crate::health::account_retriever::AccountRetriever;
use crate::error::*;
use crate::state::{
    Bank, PeachAccountRef
};

/// Information about prices for a bank or perp market.
#[derive(Clone, Debug)]
pub struct Prices {
    /// The current oracle price
    pub oracle: I80F48, // native/native

    /// A "stable" price, provided by StablePriceModel
    pub stable: I80F48, // native/native
}

impl Prices {
    // intended for tests
    pub fn new_single_price(price: I80F48) -> Self {
        Self {
            oracle: price,
            stable: price,
        }
    }

    /// The liability price to use for the given health type
    #[inline(always)]
    pub fn liab(&self, health_type: HealthType) -> I80F48 {
        match health_type {
            HealthType::Maint | HealthType::LiquidationEnd => self.oracle,
            HealthType::Init => self.oracle.max(self.stable),
        }
    }

    /// The asset price to use for the given health type
    #[inline(always)]
    pub fn asset(&self, health_type: HealthType) -> I80F48 {
        match health_type {
            HealthType::Maint | HealthType::LiquidationEnd => self.oracle,
            HealthType::Init => self.oracle.min(self.stable),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TokenInfo {
    pub token_index: TokenIndex,
    pub maint_asset_weight: I80F48,
    pub init_asset_weight: I80F48,
    pub init_scaled_asset_weight: I80F48,
    pub maint_liab_weight: I80F48,
    pub init_liab_weight: I80F48,
    pub init_scaled_liab_weight: I80F48,
    pub prices: Prices,

    /// Freely available spot balance for the token.
    ///
    /// Includes TokenPosition and free Serum3OpenOrders balances.
    /// Does not include perp upnl or Serum3 reserved amounts.
    pub balance_spot: I80F48,

    pub allow_asset_liquidation: bool,
}

impl TokenInfo {
    #[inline(always)]
    fn asset_weight(&self, health_type: HealthType) -> I80F48 {
        match health_type {
            HealthType::Init => self.init_scaled_asset_weight,
            HealthType::LiquidationEnd => self.init_asset_weight,
            HealthType::Maint => self.maint_asset_weight,
        }
    }

    #[inline(always)]
    pub fn asset_weighted_price(&self, health_type: HealthType) -> I80F48 {
        self.asset_weight(health_type) * self.prices.asset(health_type)
    }

    #[inline(always)]
    fn liab_weight(&self, health_type: HealthType) -> I80F48 {
        match health_type {
            HealthType::Init => self.init_scaled_liab_weight,
            HealthType::LiquidationEnd => self.init_liab_weight,
            HealthType::Maint => self.maint_liab_weight,
        }
    }

    #[inline(always)]
    pub fn liab_weighted_price(&self, health_type: HealthType) -> I80F48 {
        self.liab_weight(health_type) * self.prices.liab(health_type)
    }

    #[inline(always)]
    pub fn health_contribution(&self, health_type: HealthType, balance: I80F48) -> I80F48 {
        let weighted_price = if balance.is_negative() {
            self.liab_weighted_price(health_type)
        } else {
            self.asset_weighted_price(health_type)
        };
        balance * weighted_price
    }
}

/// Temporary value used during health computations
#[derive(Clone, Default)]
pub struct TokenBalance {
    /// Sum of token_info.balance_spot and perp health_unsettled_pnl balances
    pub spot_and_perp: I80F48,
}

/// Store information needed to compute account health
///
/// This is called a cache, because it extracts information from a PeachAccount and
/// the Bank, Perp, oracle accounts once and then allows computing different types
/// of health.
///
/// For compute-saving reasons, it also allows applying adjustments to the extracted
/// positions. That's often helpful for instructions that want to re-compute health
/// after having made small, well-known changes to an account. Recomputing the
/// HealthCache from scratch would be significantly more expensive.
///
/// However, there's a real risk of getting the adjustments wrong and computing an
/// inconsistent result, so particular care needs to be taken when this is done.
#[allow(unused)]
#[derive(Clone, Debug)]
pub struct HealthCache {
    pub token_infos: Vec<TokenInfo>,
    // pub(crate) serum3_infos: Vec<Serum3Info>,
    // pub(crate) perp_infos: Vec<PerpInfo>,
    #[allow(unused)]
    pub(crate) being_liquidated: bool,
}

impl HealthCache {
    
    pub fn health(&self, health_type: HealthType) -> I80F48 {
        let token_balances = self.effective_token_balances(health_type);
        let mut health = I80F48::ZERO;
        let sum = |contrib| {
            health += contrib;
        };
        self.health_sum(health_type, sum, &token_balances);
        health
    }

    pub fn token_info(&self, token_index: TokenIndex) -> Result<&TokenInfo> {
        Ok(&self.token_infos[self.token_info_index(token_index)?])
    }

    pub fn token_info_index(&self, token_index: TokenIndex) -> Result<usize> {
        self.token_infos
            .iter()
            .position(|t| t.token_index == token_index)
            .ok_or_else(|| {
                error_msg_typed!(
                    PeachError::TokenPositionDoesNotExist,
                    "token index {} not found",
                    token_index
                )
            })
    }

    pub fn has_token_info(&self, token_index: TokenIndex) -> bool {
        self.token_infos
            .iter()
            .any(|t| t.token_index == token_index)
    }

    /// Changes the cached user account token balance.
    pub fn adjust_token_balance(&mut self, bank: &Bank, change: I80F48) -> Result<()> {
        let entry_index = self.token_info_index(bank.token_index)?;
        let entry = &mut self.token_infos[entry_index];

        // Note: resetting the weights here assumes that the change has been applied to
        // the passed in bank already
        entry.init_scaled_asset_weight =
            bank.scaled_init_asset_weight(entry.prices.asset(HealthType::Init));
        entry.init_scaled_liab_weight =
            bank.scaled_init_liab_weight(entry.prices.liab(HealthType::Init));

        // Work around the fact that -((-x) * y) == x * y does not hold for I80F48:
        // We need to make sure that if balance is before * price, then change = -before
        // brings it to exactly zero.
        let removed_contribution = -change;
        entry.balance_spot -= removed_contribution;
        Ok(())
    }

    /// Returns token balances that account for spot and perp contributions
    ///
    /// Spot contributions are just the regular deposits or borrows, as well as from free
    /// funds on serum3 open orders accounts.
    ///
    /// Perp contributions come from perp positions in markets that use the token as a settle token:
    /// For these the hupnl is added to the total because that's the risk-adjusted expected to be
    /// gained or lost from settlement.
    pub fn effective_token_balances(&self, health_type: HealthType) -> Vec<TokenBalance> {
        self.effective_token_balances_internal(health_type, false)
    }

    pub(crate) fn health_sum(
        &self,
        _health_type: HealthType,
        mut _action: impl FnMut(I80F48),
        _token_balances: &[TokenBalance],
    ) {
        // for (token_info, token_balance) in self.token_infos.iter().zip(token_balances.iter()) {
        //     let contrib = token_info.health_contribution(health_type, token_balance.spot_and_perp);
        //     action(contrib);
        // }

        // let (token_max_reserved, serum3_reserved) = self.compute_serum3_reservations(health_type);
        // for (serum3_info, reserved) in self.serum3_infos.iter().zip(serum3_reserved.iter()) {
        //     let contrib = serum3_info.health_contribution(
        //         health_type,
        //         &self.token_infos,
        //         &token_balances,
        //         &token_max_reserved,
        //         reserved,
        //     );
        //     action(contrib);
        // }
    }

    /// Implementation of effective_token_balances()
    ///
    /// The ignore_negative_perp flag exists for perp_max_settle(). When it is enabled, all negative
    /// token contributions from perp markets are ignored. That's useful for knowing how much token
    /// collateral is available when limiting negative upnl settlement.
    fn effective_token_balances_internal(
        &self,
        _health_type: HealthType,
        _ignore_negative_perp: bool,
    ) -> Vec<TokenBalance> {
        let mut token_balances = vec![TokenBalance::default(); self.token_infos.len()];

        // for perp_info in self.perp_infos.iter() {
        //     let settle_token_index = self.token_info_index(perp_info.settle_token_index).unwrap();
        //     let perp_settle_token = &mut token_balances[settle_token_index];
        //     let health_unsettled = perp_info.health_unsettled_pnl(health_type);
        //     if !ignore_negative_perp || health_unsettled > 0 {
        //         perp_settle_token.spot_and_perp += health_unsettled;
        //     }
        // }

        for (token_info, token_balance) in self.token_infos.iter().zip(token_balances.iter_mut()) {
            token_balance.spot_and_perp += token_info.balance_spot;
        }

        token_balances
    }


    pub fn health_assets_and_liabs_stable_assets(
        &self,
        health_type: HealthType,
    ) -> (I80F48, I80F48) {
        self.health_assets_and_liabs(health_type, true)
    }

    pub fn health_assets_and_liabs_stable_liabs(
        &self,
        health_type: HealthType,
    ) -> (I80F48, I80F48) {
        self.health_assets_and_liabs(health_type, false)
    }

    /// Computes the account assets and liabilities marked to market.
    ///
    /// Contrary to health_assets_and_liabs, there's no health weighing or adjustment
    /// for stable prices. It uses oracle prices directly.
    ///
    /// Returns (assets, liabilities)
    pub fn assets_and_liabs(&self) -> (I80F48, I80F48) {
        let mut assets = I80F48::ZERO;
        let mut liabs = I80F48::ZERO;

        for token_info in self.token_infos.iter() {
            if token_info.balance_spot.is_negative() {
                liabs -= token_info.balance_spot * token_info.prices.oracle;
            } else {
                assets += token_info.balance_spot * token_info.prices.oracle;
            }
        }

        // for serum_info in self.serum3_infos.iter() {
        //     let quote = &self.token_infos[serum_info.quote_info_index];
        //     let base = &self.token_infos[serum_info.base_info_index];
        //     assets += serum_info.reserved_base * base.prices.oracle;
        //     assets += serum_info.reserved_quote * quote.prices.oracle;
        // }

        // for perp_info in self.perp_infos.iter() {
        //     let quote_price = self.token_infos[perp_info.settle_token_index as usize]
        //         .prices
        //         .oracle;
        //     let quote_position_value = perp_info.quote * quote_price;
        //     if perp_info.quote.is_negative() {
        //         liabs -= quote_position_value;
        //     } else {
        //         assets += quote_position_value;
        //     }

        //     let base_position_value = I80F48::from(perp_info.base_lots * perp_info.base_lot_size)
        //         * perp_info.base_prices.oracle
        //         * quote_price;
        //     if base_position_value.is_negative() {
        //         liabs -= base_position_value;
        //     } else {
        //         assets += base_position_value;
        //     }
        // }

        return (assets, liabs);
    }

    /// Loop over the token, perp, serum contributions and add up all positive values into `assets`
    /// and (the abs) of negative values separately into `liabs`. Return (assets, liabs).
    ///
    /// Due to the way token and perp positions sum before being weighted, there's some flexibility
    /// in how the sum is split up. It can either be split up such that the amount of liabs stays
    /// constant when assets change, or the other way around.
    ///
    /// For example, if assets are held stable: An account with $10 in SOL and -$12 hupnl in a
    /// SOL-settled perp market would have:
    /// - assets: $10 * SOL_asset_weight
    /// - liabs: $10 * SOL_asset_weight + $2 * SOL_liab_weight
    /// because some of the liabs are weighted lower as they are just compensating the assets.
    ///
    /// Same example if liabs are held stable:
    /// - liabs: $12 * SOL_liab_weight
    /// - assets: $10 * SOL_liab_weight
    ///
    /// The value `assets - liabs` is the health and the same in both cases.
    fn health_assets_and_liabs(
        &self,
        health_type: HealthType,
        stable_assets: bool,
    ) -> (I80F48, I80F48) {
        let mut total_assets = I80F48::ZERO;
        let mut total_liabs = I80F48::ZERO;
        let add = |assets: &mut I80F48, liabs: &mut I80F48, value: I80F48| {
            if value > 0 {
                *assets += value;
            } else {
                *liabs += -value;
            }
        };

        for token_info in self.token_infos.iter() {
            // For each token, health only considers the effective token position. But for
            // this function we want to distinguish the contribution from token deposits from
            // contributions by perp markets.
            // However, the overall weight is determined by the sum, so first collect all
            // assets parts and all liab parts and then determine the actual values.
            let mut asset_balance = I80F48::ZERO;
            let mut liab_balance = I80F48::ZERO;

            add(
                &mut asset_balance,
                &mut liab_balance,
                token_info.balance_spot,
            );

            // for perp_info in self.perp_infos.iter() {
            //     if perp_info.settle_token_index != token_info.token_index {
            //         continue;
            //     }
            //     let health_unsettled = perp_info.health_unsettled_pnl(health_type);
            //     add(&mut asset_balance, &mut liab_balance, health_unsettled);
            // }

            // The assignment to total_assets and total_liabs is a bit arbitrary.
            // As long as the (added_assets - added_liabs) = weighted(asset_balance - liab_balance),
            // the result will be consistent.
            if stable_assets {
                let asset_weighted_price = token_info.asset_weighted_price(health_type);
                let assets = asset_balance * asset_weighted_price;
                total_assets += assets;
                if asset_balance >= liab_balance {
                    // liabs partially compensate
                    total_liabs += liab_balance * asset_weighted_price;
                } else {
                    let liab_weighted_price = token_info.liab_weighted_price(health_type);
                    // the liabs fully compensate the assets and even add something extra
                    total_liabs += assets + (liab_balance - asset_balance) * liab_weighted_price;
                }
            } else {
                let liab_weighted_price = token_info.liab_weighted_price(health_type);
                let liabs = liab_balance * liab_weighted_price;
                total_liabs += liabs;
                if asset_balance >= liab_balance {
                    let asset_weighted_price = token_info.asset_weighted_price(health_type);
                    // the assets fully compensate the liabs and even add something extra
                    total_assets += liabs + (asset_balance - liab_balance) * asset_weighted_price;
                } else {
                    // assets partially compensate
                    total_assets += asset_balance * liab_weighted_price;
                }
            }
        }

        // let token_balances = self.effective_token_balances(health_type);
        // let (token_max_reserved, serum3_reserved) = self.compute_serum3_reservations(health_type);
        // for (serum3_info, reserved) in self.serum3_infos.iter().zip(serum3_reserved.iter()) {
        //     let contrib = serum3_info.health_contribution(
        //         health_type,
        //         &self.token_infos,
        //         &token_balances,
        //         &token_max_reserved,
        //         reserved,
        //     );
        //     add(&mut total_assets, &mut total_liabs, contrib);
        // }

        (total_assets, total_liabs)
    }

}


/// There are three types of health:
/// - initial health ("init"): users can only open new positions if it's >= 0
/// - maintenance health ("maint"): users get liquidated if it's < 0
/// - liquidation end health: once liquidation started (see being_liquidated), it
///   only stops once this is >= 0
///
/// The ordering is
///   init health <= liquidation end health <= maint health
///
/// The different health types are realized by using different weights and prices:
/// - init health: init weights with scaling, stable-price adjusted prices
/// - liq end health: init weights without scaling, oracle prices
/// - maint health: maint weights, oracle prices
///
#[derive(PartialEq, Copy, Clone, AnchorSerialize, AnchorDeserialize)]
pub enum HealthType {
    Init,
    Maint, // aka LiquidationStart
    LiquidationEnd,
}

/// Generate a special HealthCache for an account and its health accounts
/// where nonnegative token positions for bad oracles are skipped as well as missing banks.
///
/// This health cache must be used carefully, since it doesn't provide the actual
/// account health, just a value that is guaranteed to be less than it.
pub fn new_health_cache_skipping_missing_banks_and_bad_oracles(
    account: &PeachAccountRef,
    retriever: &impl AccountRetriever,
    now_ts: u64,
) -> Result<HealthCache> {
    new_health_cache_impl(account, retriever, now_ts, true)
}

// On `allow_skipping_banks`:
//   If (a Bank is not provided or its oracle is stale or inconfident) and the health contribution would
//   not be negative, skip it. This decreases health, but many operations are still allowed as long
//   as the decreased amount stays positive.
fn new_health_cache_impl(
    account: &PeachAccountRef,
    retriever: &impl AccountRetriever,
    now_ts: u64,
    allow_skipping_banks: bool,
) -> Result<HealthCache> {
    // token contribution from token accounts
    let mut token_infos = Vec::with_capacity(account.active_token_positions().count());

    // As a CU optimization, don't call available_banks() unless necessary
    let available_banks_opt = if allow_skipping_banks {
        Some(retriever.available_banks()?)
    } else {
        None
    };

    for (i, position) in account.active_token_positions().enumerate() {
        // Allow skipping of missing banks only if the account has a nonnegative balance
        if allow_skipping_banks {
            let bank_is_available = available_banks_opt
                .as_ref()
                .unwrap()
                .contains(&position.token_index);
            if !bank_is_available {
                require_msg_typed!(
                    position.indexed_position >= FixedWrapper::zero(),
                    PeachError::InvalidBank,
                    "the bank for token index {} is a required health account when the account has a negative balance in it",
                    position.token_index
                );
                continue;
            }
        }

        let bank_oracle_result =
            retriever.bank_and_oracle(&account.fixed.market, i, position.token_index);

        // Allow skipping of bad-oracle banks if the account has a nonnegative balance
        if allow_skipping_banks
            && bank_oracle_result.is_oracle_error()
            && position.indexed_position >= FixedWrapper::zero()
        {
            // Ignore the asset because the oracle is bad, decreasing total health
            continue;
        }
        let (bank, oracle_price) = bank_oracle_result?;

        let native = position.native(bank);
        let prices = Prices {
            oracle: oracle_price,
            stable: bank.stable_price(),
        };
        // Use the liab price for computing weight scaling, because it's pessimistic and
        // causes the most unfavorable scaling.
        let liab_price = prices.liab(HealthType::Init);

        let (maint_asset_weight, maint_liab_weight) = bank.maint_weights(now_ts);

        token_infos.push(TokenInfo {
            token_index: bank.token_index,
            maint_asset_weight,
            init_asset_weight: bank.init_asset_weight.val(),
            init_scaled_asset_weight: bank.scaled_init_asset_weight(liab_price),
            maint_liab_weight,
            init_liab_weight: bank.init_liab_weight.val(),
            init_scaled_liab_weight: bank.scaled_init_liab_weight(liab_price),
            prices,
            balance_spot: native,
            allow_asset_liquidation: bank.allows_asset_liquidation(),
        });
    }

    Ok(HealthCache {
        token_infos,
        // serum3_infos,
        // perp_infos,
        being_liquidated: account.fixed.being_liquidated(),
    })
}

/// Generate a HealthCache for an account and its health accounts.
pub fn new_health_cache(
    account: &PeachAccountRef,
    retriever: &impl AccountRetriever,
    now_ts: u64,
) -> Result<HealthCache> {
    new_health_cache_impl(account, retriever, now_ts, false)
}

