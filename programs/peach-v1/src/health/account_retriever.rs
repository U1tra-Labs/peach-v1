use anchor_lang::prelude::*;
use anchor_lang::ZeroCopy;
use fixed::types::I80F48;

// use fixed::types::I80F48;

use std::cell::Ref;
// use std::collections::HashMap;

use crate::accounts_zerocopy::*;
use crate::error::*;
use crate::state::pyth_mainnet_sol_oracle;
use crate::state::pyth_mainnet_usdc_oracle;
use crate::state::OracleAccountInfos;
// use crate::state::OracleAccountInfos;
use crate::state::{Bank, PeachAccountRef, TokenIndex};

/// This trait abstracts how to find accounts needed for the health computation.
///
/// There are different ways they are retrieved from remainingAccounts, based
/// on the instruction:
/// - FixedOrderAccountRetriever requires the remainingAccounts to be in a well
///   defined order and is the fastest. It's used where possible.
/// - ScanningAccountRetriever does a linear scan for each account it needs.
///   It needs more compute, but works when a union of bank/oracle/market accounts
///   are passed because health needs to be computed for different baskets in
///   one instruction (such as for liquidation instructions).
pub trait AccountRetriever {
    /// Returns the token indexes of the available banks. Unordered and may have duplicates.
    fn available_banks(&self) -> Result<Vec<TokenIndex>>;

    fn bank_and_oracle(
        &self,
        market: &Pubkey,
        active_token_position_index: usize,
        token_index: TokenIndex,
    ) -> Result<(&Bank, I80F48)>;
}


/// Assumes the account infos needed for the health computation follow a strict order.
///
/// 1. n_banks Bank account, in the order of account.active_token_positions() although it's
///    allowed for some of the banks (and their oracles in 2.) to be skipped
/// 2. n_banks oracle accounts, one for each bank in the same order
/// 3. PerpMarket accounts, in the order of account.perps.active_perp_positions()
/// 4. PerpMarket oracle accounts, in the order of the perp market accounts
/// 5. serum3 OpenOrders accounts, in the order of account.active_serum3_orders()
/// 6. fallback oracle accounts, order and existence of accounts is not guaranteed
pub struct FixedOrderAccountRetriever<T: KeyedAccountReader> {
    pub ais: Vec<T>,
    pub n_banks: usize,
    pub n_perps: usize,
    pub begin_perp: usize,
    pub begin_serum3: usize,
    pub now: Option<(u64, u64)>,
    pub begin_fallback_oracles: usize,
    pub usdc_oracle_index: Option<usize>,
    pub sol_oracle_index: Option<usize>,
}

/// A FixedOrderAccountRetriever with n_banks <= active_token_positions().count(),
/// depending on which banks were passed.
///
/// Note that this does not eagerly validate that the right accounts were passed. That
/// validation happens only when banks, perps etc are requested.
pub fn new_fixed_order_account_retriever_with_optional_banks<'a, 'info>(
    ais: &'a [AccountInfo<'info>],
    account: &PeachAccountRef,
    now: (u64, u64),
) -> Result<FixedOrderAccountRetriever<AccountInfoRef<'a, 'info>>> {
    // Scan for the number of banks provided
    let mut n_banks = 0;
    for ai in ais {
        if let Some((_, bank_result)) = can_load_as::<Bank>((0, ai)) {
            bank_result?;
            n_banks += 1;
        } else {
            break;
        }
    }

    let active_token_len = account.active_token_positions().count();
    require_gte!(active_token_len, n_banks);

    new_fixed_order_account_retriever_inner(ais, account, now, n_banks)
}

pub fn new_fixed_order_account_retriever_inner<'a, 'info>(
    ais: &'a [AccountInfo<'info>],
    _account: &PeachAccountRef,
    now: (u64, u64),
    n_banks: usize,
) -> Result<FixedOrderAccountRetriever<AccountInfoRef<'a, 'info>>> {
    // let active_serum3_len = account.active_serum3_orders().count();
    // let active_perp_len = account.active_perp_positions().count();
    let expected_ais = n_banks * 2; // banks + oracles
        // + active_perp_len * 2 // PerpMarkets + Oracles
        // + active_serum3_len; // open_orders
    require_msg_typed!(ais.len() >= expected_ais, PeachError::InvalidHealthAccountCount,
        "received {} accounts but expected {} ({} banks, {} bank oracles)",
        ais.len(), expected_ais,
        n_banks, n_banks ); // active_perp_len, active_perp_len, active_serum3_len
    let usdc_oracle_index = ais[..]
        .iter()
        .position(|o| o.key == &pyth_mainnet_usdc_oracle::ID);
    let sol_oracle_index = ais[..]
        .iter()
        .position(|o| o.key == &pyth_mainnet_sol_oracle::ID);

    Ok(FixedOrderAccountRetriever {
        ais: AccountInfoRef::borrow_slice(ais)?,
        n_banks,
        n_perps: 0, //active_perp_len,
        begin_perp: n_banks * 2,
        begin_serum3: n_banks * 2, // + active_perp_len * 2,
        now: Some(now),
        begin_fallback_oracles: expected_ais,
        usdc_oracle_index,
        sol_oracle_index,
    })
}

impl<T: KeyedAccountReader> FixedOrderAccountRetriever<T> {
    fn bank(
        &self,
        market: &Pubkey,
        active_token_position_index: usize,
        token_index: TokenIndex,
    ) -> Result<(usize, &Bank)> {
        // Maybe not all banks were passed: The desired bank must be at or
        // to the left of account_index and left of n_banks.
        let end_index = (active_token_position_index + 1).min(self.n_banks);
        for i in (0..end_index).rev() {
            let ai = &self.ais[i];
            let bank = ai.load_fully_unchecked::<Bank>()?;
            if bank.token_index == token_index {
                require_keys_eq!(bank.market, *market);
                return Ok((i, bank));
            }
        }
        Err(error_msg_typed!(
            PeachError::InvalidHealthAccountCount,
            "bank for token index {} not found",
            token_index
        ))
    }

    #[inline(always)]
    fn create_oracle_infos(
        &self,
        oracle_index: usize,
        fallback_key: &Pubkey,
    ) -> OracleAccountInfos<T> {
        let oracle = &self.ais[oracle_index];
        let fallback_opt = self.ais[self.begin_fallback_oracles..]
            .iter()
            .find(|ai| ai.key() == fallback_key);

        OracleAccountInfos {
            oracle,
            fallback_opt,
            usdc_opt: self.usdc_oracle_index.map(|i| &self.ais[i]),
            sol_opt: self.sol_oracle_index.map(|i| &self.ais[i]),
        }
    }
}

impl<T: KeyedAccountReader> AccountRetriever for FixedOrderAccountRetriever<T> {
    fn available_banks(&self) -> Result<Vec<TokenIndex>> {
        let mut result = Vec::with_capacity(self.n_banks);
        for bank_ai in &self.ais[0..self.n_banks] {
            let bank = bank_ai.load_fully_unchecked::<Bank>()?;
            result.push(bank.token_index);
        }
        Ok(result)
    }

    fn bank_and_oracle(
        &self,
        market: &Pubkey,
        active_token_position_index: usize,
        token_index: TokenIndex,
    ) -> Result<(&Bank, I80F48)> {
        let (bank_account_index, bank) =
            self.bank(market, active_token_position_index, token_index)?;

        let oracle_index = self.n_banks + bank_account_index;
        let oracle_acc_infos = &self.create_oracle_infos(oracle_index, &bank.fallback_oracle);
        let oracle_price_result = bank.oracle_price(oracle_acc_infos, self.now);
        let oracle_price = oracle_price_result.with_context(|| {
            format!(
                "getting oracle for bank with health account index {} and token index {}, passed account {}",
                bank_account_index,
                token_index,
                self.ais[oracle_index].key(),
            )
        })?;

        Ok((bank, oracle_price))
    }
}


/// Returns None if `ai` doesn't have the owner or discriminator for T.
/// Forwards "can't borrow" error, so it can be raised immediately.
fn can_load_as<'a, T: ZeroCopy + Owner>(
    (i, ai): (usize, &'a AccountInfo),
) -> Option<(usize, Result<Ref<'a, T>>)> {
    let load_result = ai.load::<T>();
    if load_result.is_anchor_error_with_code(ErrorCode::AccountDiscriminatorMismatch.into())
        || load_result.is_anchor_error_with_code(ErrorCode::AccountDiscriminatorNotFound.into())
        || load_result.is_anchor_error_with_code(ErrorCode::AccountOwnedByWrongProgram.into())
    {
        return None;
    }
    Some((i, load_result))
}
