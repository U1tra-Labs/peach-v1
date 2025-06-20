use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::custom_types::TokenIndex;
use crate::error::PeachError;
use crate::state::*;

#[allow(clippy::too_many_arguments)]
#[allow(unused_variables)]
pub fn token_add_bank(
    ctx: Context<TokenAddBank>,
    token_index: u16,
    bank_num: u32,
) -> Result<()> {
    let existing_bank = ctx.accounts.existing_bank.load()?;
    let mut bank = ctx.accounts.bank.load_init()?;
    let bump = ctx.bumps.bank;
    *bank = Bank::from_existing_bank(&existing_bank, ctx.accounts.vault.key(), bank_num, bump);

    let mut mint_info = ctx.accounts.mint_info.load_mut()?;
    let free_slot = mint_info
        .banks
        .iter()
        .position(|bank| bank == &Pubkey::default())
        .unwrap();
    require_eq!(bank_num as usize, free_slot);
    mint_info.banks[free_slot] = ctx.accounts.bank.key();
    mint_info.vaults[free_slot] = ctx.accounts.vault.key();

    Ok(())
}


#[derive(Accounts)]
#[instruction(token_index: u16, bank_num: u32)]
pub struct TokenAddBank<'info> {
    #[account(
        has_one = admin,
        constraint = market.load()?.is_ix_enabled(IxGate::TokenAddBank) @ PeachError::IxIsDisabled,
        constraint = market.load()?.multiple_banks_supported(),
        // Concerns are:
        // - general reaudit
        // - client support
        // - potential_serum_tokens
        constraint = market.load()?.is_testing(),
    )]
    pub market: AccountLoader<'info, Market>,
    pub admin: Signer<'info>,

    #[account(
        mint::token_program = token_program,
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        constraint = existing_bank.load()?.token_index == TokenIndex(token_index),
        has_one = market,
        has_one = mint,
    )]
    pub existing_bank: AccountLoader<'info, Bank>,

    #[account(
        init,
        // using the token_index in this seed guards against reusing it
        seeds = [b"Bank".as_ref(), market.key().as_ref(), &token_index.to_le_bytes(), &bank_num.to_le_bytes()],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<Bank>(),
    )]
    pub bank: AccountLoader<'info, Bank>,

    #[account(
        init,
        seeds = [b"Vault".as_ref(), market.key().as_ref(), &token_index.to_le_bytes(), &bank_num.to_le_bytes()],
        bump,
        token::authority = market,
        token::mint = mint,
        token::token_program = token_program,
        payer = payer
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        constraint = mint_info.load()?.token_index == TokenIndex(token_index),
        has_one = market,
        has_one = mint,
    )]
    pub mint_info: AccountLoader<'info, MintInfo>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}
