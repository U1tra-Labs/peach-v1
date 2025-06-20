use anchor_lang::prelude::*;

/// This token index is supposed to be the token that oracles quote in.
///
/// In practice this is set to the USDC token index, and that is wrong: actually
/// oracles quote in USD. Any use of this constant points to a potentially
/// incorrect assumption.
pub const QUOTE_TOKEN_INDEX: TokenIndex = TokenIndex(0);

#[account(zero_copy)]
#[repr(C)]
#[derive(Default, Debug)]
pub struct Market {
    pub creator: Pubkey,
    pub market_num: u32,
    pub admin: Pubkey,
    pub bump: u8,
    pub testing: u8,
    pub version: u8,
    pub padding: [u8; 1],
    pub deposit_limit_quote: u64,
    pub ix_gate: u128,
    pub collateral_fee_interval: u64,
    /// Padding to align struct size to multiple of 16 for zero_copy Pod
    pub reserved: [u8; 8],
}

impl Market {
    pub fn is_testing(&self) -> bool {
        self.testing == 1
    }

    pub fn is_ix_enabled(&self, ix: IxGate) -> bool {
        self.ix_gate & (1 << ix as u128) == 0
    }

    pub fn multiple_banks_supported(&self) -> bool {
        self.is_testing() || self.version > 1
    }
}

#[derive(Copy, Clone, Debug)]
pub enum IxGate {
    AccountCreate = 0,
    MarketCreate = 1,
    TokenRegister = 2,
    StubOracleCreate = 3,
    TokenDeposit = 4,
    TokenWithdraw = 5,
    TokenForceWithdraw = 6,
    MarketEdit = 7,
    MarketClose = 8,
    StubOracleSet = 9,
    StubOracleClose = 10,
    AccountEdit = 11,
    AccountExpand = 12,
    AccountSizeMigration = 13,
    AccountToggleFreeze = 14,
    AccountClose = 15,
    TokenRegisterTrustless = 16,
    TokenAddBank = 17,
    TokenUpdateIndexAndRate = 18,
    TokenEdit = 19,
    TokenDeregister = 20,
    AdminTokenWithdrawFees = 21,
    KaminoInitUserMetaData = 22,
    KaminoInitObligation = 23,
    KaminoInitObligationFarmsForReserve = 24,
    KaimnoDeposit = 25,
    KaimnoWithdraw = 26,
}

// note: using creator instead of admin, since admin can be changed
#[macro_export]
macro_rules! market_seeds {
    ( $market:expr ) => {
        &[
            b"Market".as_ref(),
            $market.creator.as_ref(),
            &$market.market_num.to_le_bytes(),
            &[$market.bump],
        ]
    };
}

pub use market_seeds;

use crate::custom_types::TokenIndex;