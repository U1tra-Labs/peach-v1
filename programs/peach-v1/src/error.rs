use anchor_lang::prelude::*;

#[error_code]
pub enum PeachError {
    #[msg("")]
    SomeError,
    #[msg("instruction is disabled")]
    IxIsDisabled
}