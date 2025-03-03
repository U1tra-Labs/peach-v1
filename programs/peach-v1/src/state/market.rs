use anchor_lang::prelude::*;

#[account(zero_copy)]
#[derive(Debug)]
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
    pub collateral_fee_interval: u64
}

impl Market {
    pub fn is_testing(&self) -> bool {
        self.testing == 1
    }

    pub fn is_ix_enabled(&self, ix: IxGate) -> bool {
        self.ix_gate & (1 << ix as u128) == 0
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
