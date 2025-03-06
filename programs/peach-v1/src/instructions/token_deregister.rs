use crate::{error::PeachError, state::*};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Token, TokenAccount};

use crate::accounts_zerocopy::LoadZeroCopyRef;
use anchor_lang::AccountsClose;

#[allow(clippy::too_many_arguments)]
pub fn token_deregister<'key, 'accounts, 'remaining: 'info, 'info>(
    ctx: Context<'key, 'accounts, 'remaining, 'info, TokenDeregister<'info>>,
) -> Result<()> {
    let mint_info = ctx.accounts.mint_info.load()?;
    {
        let total_banks = mint_info.num_banks();
        require_eq!(total_banks * 2, ctx.remaining_accounts.len());
    }

    let market = ctx.accounts.market.load()?;
    let market_seeds = market_seeds!(market);

    let dust_vault_ai = &ctx.accounts.dust_vault;

    // todo: use itertools::chunks(2)
    for i in (0..ctx.remaining_accounts.len()).step_by(2) {
        let bank_ai = &ctx.remaining_accounts[i];
        let vault_ai = &ctx.remaining_accounts[i + 1];

        require_eq!(bank_ai.key(), mint_info.banks[i / 2]);
        require_eq!(vault_ai.key(), mint_info.vaults[i / 2]);

        // todo: these checks might be superfluous, after above 2 checks
        {
            let bank = bank_ai.load::<Bank>()?;
            require_keys_eq!(bank.market, ctx.accounts.market.key());
            require_eq!(bank.token_index, mint_info.token_index);
            require_keys_eq!(bank.vault, vault_ai.key());
        }

        // transfer dust to another token account
        let amount = Account::<TokenAccount>::try_from(vault_ai).unwrap().amount;
        if amount > 0 {
            token::transfer(
                {
                    let accounts = token::Transfer {
                        from: vault_ai.to_account_info(),
                        to: dust_vault_ai.to_account_info(),
                        authority: ctx.accounts.market.to_account_info(),
                    };
                    CpiContext::new(ctx.accounts.token_program.to_account_info(), accounts)
                        .with_signer(&[market_seeds])
                },
                amount,
            )?;
        }

        // note: vault seems to need closing before bank, weird solana oddity
        let cpi_accounts = CloseAccount {
            account: vault_ai.to_account_info(),
            destination: ctx.accounts.sol_destination.to_account_info(),
            authority: ctx.accounts.market.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        token::close_account(CpiContext::new_with_signer(
            cpi_program,
            cpi_accounts,
            &[market_seeds],
        ))?;
        vault_ai.exit(ctx.program_id)?;
    }

    // Close banks in a second step, because the cpi calls above don't like when someone
    // else touches sol_destination.lamports in between.
    for i in (0..ctx.remaining_accounts.len()).step_by(2) {
        let bank_ai = &ctx.remaining_accounts[i];
        let bank_al: AccountLoader<Bank> = AccountLoader::try_from(bank_ai)?;
        bank_al.close(ctx.accounts.sol_destination.to_account_info())?;
    }

    Ok(())
}

/// In addition to these accounts, there must be remaining_accounts:
/// all n pairs of bank and its corresponding vault account for a token
#[derive(Accounts)]
pub struct TokenDeregister<'info> {
    #[account(
        has_one = admin,
        constraint = market.load()?.is_ix_enabled(IxGate::TokenDeregister) @ PeachError::IxIsDisabled,
        constraint = market.load()?.is_testing(),
    )]
    pub market: AccountLoader<'info, Market>,
    pub admin: Signer<'info>,

    // match mint info to bank
    #[account(
        mut,
        has_one = market,
        close = sol_destination
    )]
    pub mint_info: AccountLoader<'info, MintInfo>,

    #[account(mut)]
    pub dust_vault: Account<'info, TokenAccount>,

    #[account(mut)]
    /// CHECK: target for account rent needs no checks
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}
