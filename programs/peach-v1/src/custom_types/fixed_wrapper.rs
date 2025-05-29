use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};
use std::fmt;
use std::io::Write;

use fixed::types::I80F48;

#[derive(Clone, Copy, Default, Pod, Zeroable, AnchorSerialize, AnchorDeserialize)]
#[repr(transparent)]
pub struct FixedWrapper(pub i128);

impl std::fmt::Debug for FixedWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        I80F48::from_bits(self.0).fmt(f)
    }
}

impl PartialEq for FixedWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl Eq for FixedWrapper {}

impl PartialOrd for FixedWrapper {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        I80F48::from_bits(self.0).partial_cmp(&I80F48::from_bits(other.0))
    }
}
impl Ord for FixedWrapper {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        I80F48::from_bits(self.0).cmp(&I80F48::from_bits(other.0))
    }
}

impl FixedWrapper {
    pub const ZERO_BITS: i128 = 0;
    pub const ONE_BITS: i128 = 1_i128 << 48;
    pub const DELTA_BITS: i128 = 1;

    pub fn new(value: I80F48) -> Self {
        Self(value.to_bits())
    }

    pub fn val(self) -> I80F48 {
        I80F48::from_bits(self.0)
    }

    pub fn zero() -> Self {
        Self(Self::ZERO_BITS)
    }
    pub fn one() -> Self {
        Self(Self::ONE_BITS)
    }
    pub fn delta() -> Self {
        Self(Self::DELTA_BITS)
    }

    pub fn is_positive(self) -> bool {
        self.val().is_positive()
    }
    pub fn is_negative(self) -> bool {
        self.val().is_negative()
    }
    pub fn is_zero(self) -> bool {
        self.0 == 0
    }
}

impl From<I80F48> for FixedWrapper {
    fn from(value: I80F48) -> Self {
        Self::new(value)
    }
}

impl AccountSerialize for FixedWrapper {
    fn try_serialize<W: Write>(&self, writer: &mut W) -> anchor_lang::Result<()> {
        writer.write_all(&self.0.to_le_bytes()).map_err(Into::into)
    }
}

impl AccountDeserialize for FixedWrapper {
    fn try_deserialize(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        if buf.len() < std::mem::size_of::<i128>() {
            return Err(anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into());
        }
        let mut bytes = [0u8; std::mem::size_of::<i128>()];
        bytes.copy_from_slice(&buf[..std::mem::size_of::<i128>()]);
        *buf = &buf[std::mem::size_of::<i128>()..];
        Ok(Self(i128::from_le_bytes(bytes)))
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        if buf.len() < std::mem::size_of::<i128>() {
            return Err(anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into());
        }
        let mut bytes = [0u8; std::mem::size_of::<i128>()];
        bytes.copy_from_slice(&buf[..std::mem::size_of::<i128>()]);
        *buf = &buf[std::mem::size_of::<i128>()..];
        Ok(Self(i128::from_le_bytes(bytes)))
    }
}

impl fmt::Display for FixedWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Delegate to I80F48's Display implementation for a human-readable format
        write!(f, "{}", I80F48::from_bits(self.0))
    }
}
