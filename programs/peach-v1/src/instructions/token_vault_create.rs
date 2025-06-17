use anchor_lang::prelude::*;
// use anchor_spl::{token::{Mint, Token, TokenAccount}, token_interface::TokenInterface};
// use anchor_spl::token_interface::TokenAccount;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};


use crate::{custom_types::TokenIndex, error::PeachError, state::{IxGate, Market}};

const FIRST_BANK_NUM: u32 = 0;

pub fn token_vault_create(
    _ctx: Context<TokenVaultCreate>,
    token_index: u16,
) -> Result<()> {
    require_neq!(token_index, TokenIndex::MAX.0);

    Ok(())
}


#[derive(Accounts)]
#[instruction(token_index: u16)]
pub struct TokenVaultCreate<'info> {
    #[account(
        has_one = admin,
        constraint = market.load()?.is_ix_enabled(IxGate::TokenRegister) @ PeachError::IxIsDisabled,
    )]
    pub market: AccountLoader<'info, Market>,
    pub admin: Signer<'info>,

    #[account(
        mint::token_program = token_program,
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        seeds = [b"Vault".as_ref(), market.key().as_ref(), &token_index.to_le_bytes(), &FIRST_BANK_NUM.to_le_bytes()],
        bump,
        token::authority = market,
        token::mint = mint,
        token::token_program = token_program,
        payer = payer,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}
