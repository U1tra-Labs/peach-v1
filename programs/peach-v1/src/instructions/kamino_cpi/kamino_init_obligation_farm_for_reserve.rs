use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_lang::solana_program::instruction::Instruction;

use crate::constants::KAMINO_PROGRAM_ID;
use crate::state::{Market, PeachAccountFixed};
use crate::util::sighash;

pub fn kamino_init_obligation_farm_for_reserve(
    ctx: Context<KaminoInitObligationFarmsForReserve>,
    mod_num: u8,
) -> Result<()> {

    let _market = ctx.accounts.market.key();
    let account_seeds = & ctx.accounts.peach_account.load()?.pda_seeds();


    let accounts = vec![
        AccountMeta::new(ctx.accounts.payer.key(), true),
        AccountMeta::new_readonly(ctx.accounts.peach_account.key(), false),
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

    invoke_signed(
        &instruction,
        &[
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.peach_account.to_account_info(),
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
        ],
        &[&account_seeds.signer_seeds()],
    )?;

    Ok(())
}




#[derive(Accounts)]
pub struct KaminoInitObligationFarmsForReserve<'info> {
    
    #[account(mut)]
    pub payer: Signer<'info>,

    pub market: AccountLoader<'info, Market>,

    #[account(
        mut,
        has_one = market
    )]
    pub peach_account: AccountLoader<'info, PeachAccountFixed>, 

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


