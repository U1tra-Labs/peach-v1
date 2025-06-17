use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;

use crate::constants::KAMINO_PROGRAM_ID;
use crate::state::{Market, PeachAccountFixed};
use crate::util::sighash;

#[cfg(feature="mainnet")]
use anchor_lang::solana_program::program::invoke;

pub fn kamino_init_obligation_farm_for_reserve(
    ctx: Context<KaminoInitObligationFarmsForReserve>,
    mod_num: u8,
) -> Result<()> {

    let accounts = vec![
        AccountMeta::new(ctx.accounts.owner.key(), true), // payer
        AccountMeta::new_readonly(ctx.accounts.owner.key(), false), // owner
        AccountMeta::new(ctx.accounts.obligation.key(), false),
        AccountMeta::new_readonly(ctx.accounts.lending_market_authority.key(), false),
        AccountMeta::new(ctx.accounts.reserve.key(), false),
        AccountMeta::new(ctx.accounts.reserve_farm_state.key(), false),
        AccountMeta::new(ctx.accounts.obligation_farm.key(), false),
        AccountMeta::new_readonly(ctx.accounts.lending_market.key(), false),
        AccountMeta::new_readonly(ctx.accounts.farms_program.key(), false),
        AccountMeta::new_readonly(ctx.accounts.rent.key(), false),
        AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
        AccountMeta::new_readonly(ctx.accounts.kamino_program.key(), false), // Not sure if this is needed
    ];

    let discriminator = sighash("global", "init_obligation_farms_for_reserve");

    let mut data = discriminator.to_vec();
    data.extend_from_slice(&mod_num.to_le_bytes());

    let instruction = Instruction {
        program_id: KAMINO_PROGRAM_ID,
        accounts,
        data,
    };

    let account_infos = &[
            ctx.accounts.owner.to_account_info(), // payer
            ctx.accounts.owner.to_account_info(), // owner
            ctx.accounts.obligation.clone(),
            ctx.accounts.lending_market_authority.clone(),
            ctx.accounts.reserve.clone(),
            ctx.accounts.reserve_farm_state.clone(),
            ctx.accounts.obligation_farm.clone(),
            ctx.accounts.lending_market.clone(),
            ctx.accounts.farms_program.clone(),
            ctx.accounts.rent.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.kamino_program.clone(), 
        ];

    #[cfg(feature = "mainnet")]
    {
        invoke(&instruction, account_infos)?;
    }

    #[cfg(not(feature = "mainnet"))]
    {
        msg!("skipped CPI invoke for testnet/devnet");
    }

    Ok(())
}




#[derive(Accounts)]
pub struct KaminoInitObligationFarmsForReserve<'info> {

    pub market: AccountLoader<'info, Market>,

    #[account(
        mut,
        has_one = market
    )]
    pub peach_account: AccountLoader<'info, PeachAccountFixed>, 

    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(mut)]
    /// CHECK: Verified by Kamino program
    pub obligation: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: Verified via PDA constraints
    pub lending_market_authority: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: Verified by Kamino program
    pub reserve: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: Verified by Kamino program
    pub reserve_farm_state: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: Verified by Kamino program
    pub obligation_farm: AccountInfo<'info>,

    /// CHECK: Verified by Kamino program
    pub lending_market: AccountInfo<'info>,

    /// Kamino Farms program
    /// CHECK: Verified by Kamino program
    pub farms_program: AccountInfo<'info>,

    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,

    /// CHECK: Kamino program ID
    pub kamino_program: AccountInfo<'info>,
}


