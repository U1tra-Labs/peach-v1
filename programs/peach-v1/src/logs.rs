use anchor_lang::prelude::*;
use borsh::BorshSerialize;

#[inline(never)] // ensure fresh stack frame
pub fn emit_stack<T: anchor_lang::Event>(e: T) {
    use std::io::{Cursor, Write};

    // stack buffer, stack frames are 4kb
    let mut buffer = [0u8; 3000];

    let mut cursor = Cursor::new(&mut buffer[..]);
    cursor.write_all(&T::DISCRIMINATOR).unwrap();
    e.serialize(&mut cursor)
        .expect("event must fit into stack buffer");

    let pos = cursor.position() as usize;
    anchor_lang::solana_program::log::sol_log_data(&[&buffer[..pos]]);
}

#[event]
pub struct TokenMetaDataLogV2 {
    pub market: Pubkey,
    pub mint: Pubkey,
    pub token_index: u16,
    pub mint_decimals: u8,
    pub oracle: Pubkey,
    pub fallback_oracle: Pubkey,
    pub mint_info: Pubkey,
}

#[event]
pub struct TokenBalanceLog {
    pub peach_market: Pubkey,
    pub peach_account: Pubkey,
    pub token_index: u16,       // IDL doesn't support usize
    pub indexed_position: i128, // on client convert i128 to I80F48 easily by passing in the BN to I80F48 ctor
    pub deposit_index: i128,    // I80F48
    pub borrow_index: i128,     // I80F48
}

#[event]
pub struct DepositLog {
    pub peach_market: Pubkey,
    pub peach_account: Pubkey,
    pub signer: Pubkey,
    pub token_index: u16,
    pub quantity: u64,
    pub price: i128, // I80F48
}

#[event]
pub struct DeactivateTokenPositionLog {
    pub peach_market: Pubkey,
    pub peach_account: Pubkey,
    pub token_index: u16,
    pub cumulative_deposit_interest: f64,
    pub cumulative_borrow_interest: f64,
}

#[derive(PartialEq, Copy, Clone, Debug, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]
pub enum LoanOriginationFeeInstruction {
    Unknown,
    LiqTokenBankruptcy,
    LiqTokenWithToken,
    // Serum3LiqForceCancelOrders,
    // Serum3PlaceOrder,
    // Serum3SettleFunds,
    TokenWithdraw,
    // TokenConditionalSwapTrigger,
}

#[event]
pub struct WithdrawLoanLog {
    pub peach_market: Pubkey,
    pub peach_account: Pubkey,
    pub token_index: u16,
    pub loan_amount: i128,
    pub loan_origination_fee: i128,
    pub instruction: LoanOriginationFeeInstruction,
    pub price: Option<i128>, // Ideally would log price everywhere but in serum3_settle_funds oracle is not a passed in account
}

#[event]
pub struct WithdrawLog {
    pub peach_market: Pubkey,
    pub peach_account: Pubkey,
    pub signer: Pubkey,
    pub token_index: u16,
    pub quantity: u64,
    pub price: i128, // I80F48
}

#[event]
pub struct TokenCollateralFeeLog {
    pub peach_market: Pubkey,
    pub peach_account: Pubkey,
    pub token_index: u16,
    pub asset_usage_fraction: i128,
    pub fee: i128,
    pub price: i128,
}

#[event]
pub struct ForceWithdrawLog {
    pub peach_market: Pubkey,
    pub peach_account: Pubkey,
    pub token_index: u16,
    pub quantity: u64,
    pub price: i128, // I80F48
    pub to_token_account: Pubkey,
}

#[event]
pub struct UpdateIndexLog {
    pub peach_market: Pubkey,
    pub token_index: u16,
    pub deposit_index: i128,   // I80F48
    pub borrow_index: i128,    // I80F48
    pub avg_utilization: i128, // I80F48
    pub price: i128,           // I80F48
    pub stable_price: i128,    // I80F48
    pub collected_fees: i128,  // I80F48
    pub loan_fee_rate: i128,   // I80F48
    pub total_borrows: i128,
    pub total_deposits: i128,
    pub borrow_rate: i128,
    pub deposit_rate: i128,
}

#[event]
pub struct UpdateRateLog {
    pub peach_market: Pubkey,
    pub token_index: u16,
    // contrary to v1 these do not have curve_scaling factored in!
    pub rate0: i128,    // I80F48
    pub util0: i128,    // I80F48
    pub rate1: i128,    // I80F48
    pub util1: i128,    // I80F48
    pub max_rate: i128, // I80F48
    pub curve_scaling: f64,
    pub target_utilization: f32,
}
