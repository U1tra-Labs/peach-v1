use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hash;

use crate::error::PeachError;

#[inline(never)]
pub fn fill_from_str<const N: usize>(name: &str) -> Result<[u8; N]> {
    let name_bytes = name.as_bytes();
    require!(name_bytes.len() <= N,  PeachError::SomeError);
    let mut name_ = [0u8; N];
    name_[..name_bytes.len()].copy_from_slice(name_bytes);
    Ok(name_)
}

#[inline(never)]
pub fn format_zero_terminated_utf8_bytes(
    name: &[u8],
    fmt: &mut std::fmt::Formatter,
) -> std::result::Result<(), std::fmt::Error> {
    fmt.write_str(
        std::str::from_utf8(name)
            .unwrap()
            .trim_matches(char::from(0)),
    )
}

// Returns (now_ts, now_slot)
pub fn clock_now() -> (u64, u64) {
    let clock = Clock::get().unwrap();
    (clock.unix_timestamp.try_into().unwrap(), clock.slot)
}

/// Generates an 8-byte sighash from a given namespace and function name.
///
/// # Arguments
///
/// * `namespace` - The namespace of the function (e.g., module name).
/// * `name` - The function name.
///
/// # Returns
///
/// An 8-byte sighash derived from the hash of `"namespace:function"`.
pub fn sighash(namespace: &str, name: &str) -> [u8; 8] {
    let preimage = format!("{}:{}", namespace, name);
    let mut sighash = [0u8; 8];
    sighash.copy_from_slice(&hash(preimage.as_bytes()).to_bytes()[..8]);
    sighash
}
