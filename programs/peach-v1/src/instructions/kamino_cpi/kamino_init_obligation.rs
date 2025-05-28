

use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_lang::solana_program::instruction::Instruction;

use crate::constants::KAMINO_PROGRAM_ID;
use crate::state::{Market, PeachAccountFixed};
use crate::util::sighash;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct InitObligationArgs {
    pub tag: u8,
    pub id: u8,
}

pub fn kamino_init_obligation(
    ctx: Context<KaminoInitObligation>,
    args: InitObligationArgs,
) -> Result<()> {

     // it's crucial to always verify the program_id, accounts, and data passed into the CPI
    //  let market = ctx.accounts.market.key();
    //  let signer_seeds = &[
    //      ctx.accounts.payer.key.as_ref(), 
    //      market.as_ref(),
    //      &[ctx.bumps.account],
    //  ];
    let _account_seeds = & ctx.accounts.peach_account.load()?.pda_seeds();
 
    let accounts = vec![
        AccountMeta::new_readonly(ctx.accounts.payer.key(), true), 
        AccountMeta::new(ctx.accounts.payer.key(), true), 
        AccountMeta::new(ctx.accounts.obligation.key(), false), 
        AccountMeta::new_readonly(ctx.accounts.lending_market.key(), false), 
        AccountMeta::new_readonly(ctx.accounts.seed_one_account.key(), false), 
        AccountMeta::new_readonly(ctx.accounts.seed_two_account.key(), false),
        AccountMeta::new_readonly(ctx.accounts.owner_user_metadata.key(), false),
        AccountMeta::new_readonly(ctx.accounts.rent.key(), false), 
        AccountMeta::new_readonly(ctx.accounts.system_program.key(), false), 
        AccountMeta::new_readonly(ctx.accounts.kamino_program.key(), false), // Not sure if this is needed
    ];

    let discriminator = sighash("global", "init_obligation");

    let mut data = discriminator.to_vec();
    data.extend_from_slice(&args.try_to_vec()?); 


    let instruction = Instruction {
        program_id: KAMINO_PROGRAM_ID,
        accounts,
        data,
    };

    invoke_signed(
        &instruction,
        &[
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.obligation.clone(),
            ctx.accounts.lending_market.clone(),
            ctx.accounts.seed_one_account.clone(),
            ctx.accounts.seed_two_account.clone(),
            ctx.accounts.owner_user_metadata.clone(),   
            ctx.accounts.rent.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.kamino_program.clone(), 
        ],
        &[]
        // &[&account_seeds.signer_seeds()],
    )?;

    Ok(())
}


#[derive(Accounts)]
pub struct KaminoInitObligation<'info> {

    #[account(mut)]
    pub payer: Signer<'info>,

    pub market: AccountLoader<'info, Market>,

    #[account(
        mut,
        has_one = market,
    )]
    pub peach_account: AccountLoader<'info, PeachAccountFixed>, 
        
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
    pub system_program: Program<'info, System>, 

     /// CHECK: Kamino program ID
     pub kamino_program: AccountInfo<'info>,
    
}