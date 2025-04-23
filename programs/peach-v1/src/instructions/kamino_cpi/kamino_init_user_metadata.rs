use anchor_lang::prelude::*;
use crate::state::{Market, PeachAccountFixed};
use crate::util::sighash;
use crate::KAMINO_PROGRAM_ID_MAINNET;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_lang::solana_program::instruction::Instruction;
use crate::error::PeachError;    

pub fn kamino_init_user_metadata(
    ctx: Context<KaminoInitUserMetaData>,
    user_lookup_table_account: Pubkey,
) -> Result<()> {

    let account = & ctx.accounts.account.key();
    let market = ctx.accounts.market.key();

    let seeds = &[b"user_meta", account.as_ref()];
    let (expected_pda, _bump) = Pubkey::find_program_address(seeds, &KAMINO_PROGRAM_ID_MAINNET);

    require!(
        ctx.accounts.user_metadata.key() == expected_pda,
        PeachError::InvalidKaminoUserMetadataAccount
    );

    let signer_seeds = &[
        ctx.accounts.payer.key.as_ref(), 
        market.as_ref(),
        &[ctx.bumps.account],
    ];

    let accounts = vec![
        AccountMeta::new_readonly(ctx.accounts.account.key(), true), 
        AccountMeta::new(ctx.accounts.payer.key(), true), 
        AccountMeta::new(ctx.accounts.user_metadata.key(), false), 
        AccountMeta::new_readonly(ctx.accounts.referrer_user_metadata.key(), false), 
        AccountMeta::new_readonly(ctx.accounts.rent.key(), false), 
        AccountMeta::new_readonly(ctx.accounts.system_program.key(), false), 
    ];

    let discriminator = sighash("global", "init_user_metadata");

    let mut data = discriminator.to_vec();
    data.extend_from_slice(&user_lookup_table_account.to_bytes());

    let instruction = Instruction {
        program_id: KAMINO_PROGRAM_ID_MAINNET,
        accounts,
        data,
    };

    invoke_signed(
        &instruction,
        &[
            ctx.accounts.account.to_account_info(),
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.user_metadata.clone(),
            ctx.accounts.referrer_user_metadata.clone(),
            ctx.accounts.rent.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        &[signer_seeds],    
    )?;

    Ok(())
}


#[derive(Accounts)]
pub struct KaminoInitUserMetaData<'info> {

    #[account(mut)]
    pub payer: Signer<'info>,

    pub market: AccountLoader<'info, Market>,

    #[account(
        mut,
        seeds = [payer.key().as_ref(), market.key().as_ref()],
        bump,
    )]
    pub account: AccountLoader<'info, PeachAccountFixed>,  

    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut)]
    /// CHECK: will be verified by process method
    pub user_metadata: AccountInfo<'info>,
    /// CHECK: Verified by Kamino program
    pub referrer_user_metadata: AccountInfo<'info>,

    pub system_program: Program<'info, System>, 
    pub rent: Sysvar<'info, Rent>,     
}