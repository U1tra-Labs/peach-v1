use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};

use fixed::types::I80F48;
use static_assertions::const_assert_eq;
use std::mem::size_of;

use crate::{custom_types::{fixed_wrapper::FixedWrapper, TokenIndex}, state::*};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, AnchorSerialize, AnchorDeserialize, Pod, Zeroable)]
pub struct TokenPosition {
    // TODO: Why did we have deposits and borrows as two different values
    //       if only one of them was allowed to be != 0 at a time?
    // todo: maybe we want to split collateral and lending?
    // todo: see https://github.com/blockworks-foundation/mango-v4/issues/1
    // todo: how does ftx do this?
    /// The deposit_index (if positive) or borrow_index (if negative) scaled position
    pub indexed_position: FixedWrapper,    // Offset 0, Size 16. Ends 16.

    // pub indexed_position_kamino: FixedWrapper, // Offset 16, Size 16. Ends 32.
    /// index into Market.tokens
    pub token_index: TokenIndex,                // Offset 16, Size 2.
    /// incremented when a market requires this position to stay alive
    pub in_use_count: u16,                      // Offset 18, Size 2.
    /// 0 = not a kamino position
    /// 1 = kamino position
    pub is_kamino_position: u8,                 // Offset 20, Size 1.
    pub padding: [u8; 3],                       // Offset 21, Size 3. Ends 24.
    // bookkeeping variable for onchain interest calculation
    // either deposit_index or borrow_index at last indexed_position change
    pub _internal_padding_align_prev_idx: [u8; 8], // Offset 24, Size 8. Ends 32.
    pub previous_index: FixedWrapper,      // Offset 32, Size 16. Ends 48.
    // (Display only)
    // Cumulative deposit interest in token native units
    pub cumulative_deposit_interest: f64,       // Offset 48, Size 8. Ends 56.
    // (Display only)
    // Cumulative borrow interest in token native units
    pub cumulative_borrow_interest: f64,      // Offset 56, Size 8. Ends 64.
    // Explicit padding to make the struct size a multiple of its alignment (16 bytes for FixedWrapper)
    // Current data size is 56 bytes. To make it 64 bytes (next multiple of 16):
    pub _struct_padding_for_pod: [u8; 16],        // Offset 64, Size 16. Ends 80.
    // #[derivative(Debug = "ignore")]
    // pub reserved: [u8; 128],
}
// const_assert_eq!(
//     size_of::<TokenPosition>(),
//     16 + 2 + 2 + 4 + 16 + 8 + 8 + 128
// );
// const_assert_eq!(size_of::<TokenPosition>(), 184);
// const_assert_eq!(size_of::<TokenPosition>(), 80);
const_assert_eq!(size_of::<TokenPosition>() % 16, 0);

impl Default for TokenPosition {
    fn default() -> Self {
        TokenPosition {
            indexed_position: FixedWrapper::zero(),
            // indexed_position_kamino: FixedWrapper::zero(),
            token_index: TokenIndex::MAX,
            is_kamino_position: 0,
            in_use_count: 0,
            padding: [0; 3], // Explicitly initialize, though Default::default() is fine for [u8;N]
            _internal_padding_align_prev_idx: [0; 8],
            previous_index: FixedWrapper::zero(),
            cumulative_deposit_interest: 0.0,
            cumulative_borrow_interest: 0.0,
            _struct_padding_for_pod: [0; 16],
            // reserved: [0; 128],
        }
    }
}

impl TokenPosition {
    pub fn is_active(&self) -> bool {
        self.token_index != TokenIndex::MAX
    }

    pub fn is_active_for_token(&self, token_index: TokenIndex) -> bool {
        self.token_index == token_index
    }

    pub fn is_kamino_position(&self) -> bool {
        self.is_kamino_position == 1
    }

    pub fn native(&self, bank: &Bank) -> I80F48 {
        if self.indexed_position.is_positive() {
            self.indexed_position.val() * bank.deposit_index.val()
        } else {
            self.indexed_position.val() * bank.borrow_index.val()
        }
    }

    // pub fn kamino(&self, bank: &Bank) -> I80F48 {
    //     if self.indexed_position_kamino.is_positive() {
    //         self.indexed_position_kamino.val() * bank.deposit_index.val()
    //     } else {
    //         self.indexed_position_kamino.val() * bank.borrow_index.val()
    //     }
    // }

    #[cfg(feature = "client")]
    pub fn ui(&self, bank: &Bank) -> I80F48 {
        if self.indexed_position.is_positive() {
            (self.indexed_position.val() * bank.deposit_index.val())
                / I80F48::from_num(10u64.pow(bank.mint_decimals as u32))
        } else {
            (self.indexed_position.val() * bank.borrow_index.val())
                / I80F48::from_num(10u64.pow(bank.mint_decimals as u32))
        }
    }

    pub fn is_in_use(&self) -> bool {
        self.in_use_count > 0
    }

    pub fn increment_in_use(&mut self) {
        self.in_use_count += 1; // panic on overflow
    }

    pub fn decrement_in_use(&mut self) {
        self.in_use_count = self.in_use_count.saturating_sub(1);
    }
}