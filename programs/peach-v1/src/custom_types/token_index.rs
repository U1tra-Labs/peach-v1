use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};

#[derive(
    AnchorSerialize,
    AnchorDeserialize,
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Pod,
    Zeroable,
    Hash,
)]
#[repr(transparent)]
pub struct TokenIndex(pub u16);

impl TokenIndex {
    pub const MAX: Self = TokenIndex(u16::MAX);
}

impl std::fmt::Display for TokenIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}