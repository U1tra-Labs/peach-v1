use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};

#[repr(transparent)]
#[derive(Clone, Copy, Pod, Zeroable, Default, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct F32Bytes(pub [u8; 4]);

impl F32Bytes {
    pub fn new(val: f32) -> Self {
        Self(val.to_le_bytes())
    }
    pub fn val(&self) -> f32 {
        f32::from_le_bytes(self.0)
    }
}