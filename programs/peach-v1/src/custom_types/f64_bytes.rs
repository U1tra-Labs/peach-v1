use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};

#[repr(transparent)]
#[derive(Clone, Copy, Pod, Zeroable, Default, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct F64Bytes(pub [u8; 8]);

impl F64Bytes {
    pub fn new(val: f64) -> Self {
        Self(val.to_le_bytes())
    }
    pub fn val(&self) -> f64 {
        f64::from_le_bytes(self.0)
    }
}