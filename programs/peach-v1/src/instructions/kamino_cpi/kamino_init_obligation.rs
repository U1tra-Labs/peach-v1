

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;

use crate::constants::KAMINO_PROGRAM_ID;
use crate::state::{Market, PeachAccountFixed};
use crate::util::sighash;

#[cfg(feature="mainnet")]
use anchor_lang::solana_program::program::invoke;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct InitObligationArgs {
    pub tag: u8,
    pub id: u8,
}

pub fn kamino_init_obligation(
    ctx: Context<KaminoInitObligation>,
    args: InitObligationArgs,
) -> Result<()> {
 
    let accounts = vec![
        AccountMeta::new_readonly(ctx.accounts.owner.key(), true), // obligation_owner
        AccountMeta::new(ctx.accounts.owner.key(), true),  // fee_payer
        AccountMeta::new(ctx.accounts.obligation.key(), false), 
        AccountMeta::new_readonly(ctx.accounts.lending_market.key(), false), 
        AccountMeta::new_readonly(ctx.accounts.seed_one_account.key(), false), 
        AccountMeta::new_readonly(ctx.accounts.seed_two_account.key(), false),
        AccountMeta::new_readonly(ctx.accounts.owner_user_metadata.key(), false),
        AccountMeta::new_readonly(ctx.accounts.rent.key(), false), 
        AccountMeta::new_readonly(ctx.accounts.system_program.key(), false), 
    ];

    let discriminator = sighash("global", "init_obligation");

    let mut data = discriminator.to_vec();
    data.extend_from_slice(&args.try_to_vec()?); 


    let instruction = Instruction {
        program_id: KAMINO_PROGRAM_ID,
        accounts,
        data,
    };

    let account_infos = &[
        ctx.accounts.owner.to_account_info(), // obligation_owner
        ctx.accounts.owner.to_account_info(), // fee_payer
        ctx.accounts.obligation.clone(),
        ctx.accounts.lending_market.clone(),
        ctx.accounts.seed_one_account.clone(),
        ctx.accounts.seed_two_account.clone(),
        ctx.accounts.owner_user_metadata.clone(),   
        ctx.accounts.rent.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
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
pub struct KaminoInitObligation<'info> {

    pub market: AccountLoader<'info, Market>,

    #[account(
        mut,
        has_one = market,
    )]
    pub peach_account: AccountLoader<'info, PeachAccountFixed>, 

    #[account(mut)]
    pub owner: Signer<'info>,
        
    /// CHECK: Verified by Kamino program
    #[account(mut)]
    pub obligation: AccountInfo<'info>,

    /// CHECK: Verified by Kamino program
    pub lending_market: AccountInfo<'info>,

    /// CHECK: Verified by Kamino program
    pub seed_one_account: AccountInfo<'info>,

    /// CHECK: Verified by Kamino program
    pub seed_two_account: AccountInfo<'info>,

    /// CHECK: Verified by Kamino program
    pub owner_user_metadata: AccountInfo<'info>,

    pub rent: Sysvar<'info, Rent>,     
    pub system_program: Program<'info, System>
}