use anchor_lang::*;

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
