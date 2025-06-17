use anchor_lang::prelude::*;
use crate::constants::KAMINO_PROGRAM_ID;
use crate::state::{Market, PeachAccountFixed};
use crate::util::sighash;
use anchor_lang::solana_program::instruction::Instruction;
use crate::error::PeachError; 

#[cfg(feature="mainnet")]
use anchor_lang::solana_program::program::invoke;

pub fn kamino_init_user_metadata(
    ctx: Context<KaminoInitUserMetaData>,
    user_lookup_table_account: Pubkey,
) -> Result<()> {

    let account = & ctx.accounts.owner.key();

    let seeds = &[b"user_meta", account.as_ref()];
    let (expected_pda, _bump) = Pubkey::find_program_address(seeds, &KAMINO_PROGRAM_ID);

    require!(
        ctx.accounts.user_metadata.key() == expected_pda,
        PeachError::InvalidKaminoUserMetadataAccount
    );

    let accounts = vec![
        AccountMeta::new_readonly(ctx.accounts.owner.key(), true), 
        AccountMeta::new(ctx.accounts.owner.key(), true), 
        AccountMeta::new(ctx.accounts.user_metadata.key(), false), 
        AccountMeta::new_readonly(ctx.accounts.referrer_user_metadata.key(), false),
        AccountMeta::new_readonly(ctx.accounts.rent.key(), false), 
        AccountMeta::new_readonly(ctx.accounts.system_program.key(), false), 
    ];

    let discriminator = sighash("global", "init_user_metadata");

    let mut data = discriminator.to_vec();
    data.extend_from_slice(&user_lookup_table_account.to_bytes());

    let instruction = Instruction {
        program_id: KAMINO_PROGRAM_ID,
        accounts,
        data,
    };

    let account_infos = &[
        ctx.accounts.owner.to_account_info(),
        ctx.accounts.owner.to_account_info(),
        ctx.accounts.user_metadata.clone(),
        ctx.accounts.referrer_user_metadata.clone(), // This is Optional
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
pub struct KaminoInitUserMetaData<'info> {

    pub market: AccountLoader<'info, Market>,

    #[account(
        mut,
        has_one = market
    )]
    pub peach_account: AccountLoader<'info, PeachAccountFixed>, 

    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(mut)]
    /// CHECK: will be verified by process method
    pub user_metadata: AccountInfo<'info>,
    /// CHECK: Verified by Kamino program
    pub referrer_user_metadata: AccountInfo<'info>,
    
    pub rent: Sysvar<'info, Rent>,     
    pub system_program: Program<'info, System>, 
}