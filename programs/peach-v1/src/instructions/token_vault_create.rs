use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::{error::PeachError, state::{IxGate, Market}};
use crate::state::*;

const FIRST_BANK_NUM: u32 = 0;

pub fn token_vault_create(
    _ctx: Context<TokenVaultCreate>,
    token_index: TokenIndex,
) -> Result<()> {
    require_neq!(token_index, TokenIndex::MAX);

    Ok(())
}


#[derive(Accounts)]
#[instruction(token_index: TokenIndex)]
pub struct TokenVaultCreate<'info> {
    #[account(
        has_one = admin,
        constraint = market.load()?.is_ix_enabled(IxGate::TokenRegister) @ PeachError::IxIsDisabled,
    )]
    pub market: AccountLoader<'info, Market>,
    pub admin: Signer<'info>,

    pub mint: Account<'info, Mint>,

    #[account(
        init,
        seeds = [b"Vault".as_ref(), market.key().as_ref(), &token_index.0.to_le_bytes(), &FIRST_BANK_NUM.to_le_bytes()],
        bump,
        token::authority = market,
        token::mint = mint,
        payer = payer,
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}
