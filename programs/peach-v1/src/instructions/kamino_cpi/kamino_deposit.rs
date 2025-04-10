
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::{program::invoke_signed, sysvar};
use anchor_spl::token::Token;
use anchor_spl::{
    associated_token::AssociatedToken, 
    token_interface::{
        Mint, 
        TokenAccount, 
        TokenInterface, 
    }
};
use crate::error::PeachError;
use crate::util::sighash;
use crate::state::{Market, PeachAccountFixed};
use crate::constants::KAMINO_PROGRAM_ID_MAINNET;

pub fn kamino_deposit<'info>(
    ctx: Context<'_, '_, '_, 'info, DepositKamino<'info>>,
    deposit_amount: u64, 
    // protocol_index: u8,
) -> Result<()> {
    
    require!(deposit_amount > 0, PeachError::InvalidAmount);

    // msg!("Deposit amount: {:?}", deposit_amount);    
    // msg!("Deposit mint: {:?}", ctx.accounts.mint.key()); 

    // msg!("CPI Tranfer: Kamino - Deposit");

    let (kamino_reserve_liquidity_usdc_supply_pda, _bump) = Pubkey::find_program_address(
        &[
            b"reserve_liq_supply",
            ctx.accounts.lending_market.key().as_ref(),
            ctx.accounts.mint.key().as_ref(),
        ],
        &KAMINO_PROGRAM_ID_MAINNET,  // Kamino program ID here!
    );

    require_keys_eq!(
        kamino_reserve_liquidity_usdc_supply_pda,
        ctx.accounts.kamino_reserve_liquidity_usdc_supply.key(),
        PeachError::InvalidKaminoReserveLiquiditySupplyAccount
    );


    let accounts = vec![
        AccountMeta::new(ctx.accounts.user_account.key(), true), 
        AccountMeta::new(ctx.accounts.obligation.key(), false), 
        AccountMeta::new_readonly(ctx.accounts.lending_market.key(), false), 
        AccountMeta::new_readonly(ctx.accounts.lending_market_authority.key(), false), 
        AccountMeta::new(ctx.accounts.kamino_reserve.key(), false),     
        AccountMeta::new(ctx.accounts.mint.key(), false),
        AccountMeta::new(ctx.accounts.kamino_reserve_liquidity_usdc_supply.key(), false),
        AccountMeta::new(ctx.accounts.kamino_collateral_mint.key(), false),
        AccountMeta::new(ctx.accounts.kamino_destination_deposit_collateral.key(), false),
        AccountMeta::new(ctx.accounts.user_token_account.key(), false),
        AccountMeta::new(ctx.accounts.user_kamino_reserve_usdc_token_account.key(), false),
        AccountMeta::new_readonly(ctx.accounts.collateral_token_program.key(), false),
        AccountMeta::new_readonly(ctx.accounts.liquidity_token_program.key(), false),
        AccountMeta::new_readonly(ctx.accounts.instructions_sysvar.key(), false),
        AccountMeta::new_readonly(ctx.accounts.kamino_program.key(), false),
        AccountMeta::new(ctx.accounts.kamino_reserve_farm_state.key(), false),
        AccountMeta::new_readonly(ctx.accounts.farms_program.key(), false),
    ];

    let discriminator = sighash("global", "deposit_reserve_liquidity_and_obligation_collateral_v2");

    let mut data = discriminator.to_vec();
    data.extend_from_slice(&deposit_amount.to_le_bytes());


    let kamino_deposit_ix = Instruction {
        program_id: ctx.accounts.kamino_program.key(),
        accounts,
        data,
    };


    let lending_hub_key = ctx.accounts.market.key();
    let signer_seeds = &[
        ctx.accounts.signer.key.as_ref(), 
        lending_hub_key.as_ref(),
        &[ctx.bumps.user_account],
    ];

    
    invoke_signed(
        &kamino_deposit_ix,
        &[
            ctx.accounts.user_account.to_account_info(),
            ctx.accounts.obligation.clone(),
            ctx.accounts.lending_market.clone(),
            ctx.accounts.lending_market_authority.clone(),
            ctx.accounts.kamino_reserve.clone(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.kamino_reserve_liquidity_usdc_supply.to_account_info(),
            ctx.accounts.kamino_collateral_mint.to_account_info(),
            ctx.accounts.kamino_destination_deposit_collateral.to_account_info(),
            ctx.accounts.user_token_account.to_account_info(),
            ctx.accounts.user_kamino_reserve_usdc_token_account.to_account_info(),
            ctx.accounts.collateral_token_program.to_account_info(),
            ctx.accounts.liquidity_token_program.to_account_info(),
            ctx.accounts.instructions_sysvar.to_account_info(), 
            ctx.accounts.kamino_program.clone(),
            ctx.accounts.kamino_reserve_farm_state.clone(),
            ctx.accounts.farms_program.clone(),
        ],
        &[signer_seeds],
    )?;

    // TODO: Update PeachAccountFixed with the new protocol assignment
    
    // let mut user_account = ctx.accounts.user_account.load_mut()?;  
    // require!(
    //     PROTOCOL_KAMINO == protocol_index,
    //     PeachError::LendingProtocolMismatch
    // ); 
    // user_account.update_protocol_assignment(&ctx.accounts.mint.key(), PROTOCOL_KAMINO)?;


    Ok(())
}


#[derive(Accounts)]
pub struct DepositKamino<'info> {

    #[account(
        mut,
        address = user_account.load()?.owner,
    )]
    pub signer: Signer<'info>,

    /// CHECK: Verified by Kamino program
    #[account(mut)]
    pub obligation: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [signer.key().as_ref(), market.key().as_ref()],
        bump,
    )]
    pub user_account: AccountLoader<'info, PeachAccountFixed>,

    pub market: AccountLoader<'info, Market>,

    #[account(mut)]
    pub kamino_collateral_mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = kamino_collateral_mint,
        associated_token::authority = user_account,
        associated_token::token_program = collateral_token_program,
    )]
    pub user_kamino_reserve_usdc_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = mint,
        token::authority = user_account,
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(mut)]
    /// CHECK: Verified by Kamino program
    pub kamino_reserve: AccountInfo<'info>,
    /// CHECK: Verified by Kamino program
    pub lending_market: AccountInfo<'info>,
    /// CHECK: Verified via PDA constraints
    pub lending_market_authority: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: Verified by Kamino program / process method
    pub kamino_destination_deposit_collateral: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: Verified by Kamino program / process method
    pub kamino_reserve_liquidity_usdc_supply: AccountInfo<'info>,


    #[account(address = KAMINO_PROGRAM_ID_MAINNET)]
    /// CHECK: Kamino program ID
    pub kamino_program: AccountInfo<'info>, 
    /// Kamino Farms program
    /// CHECK: Verified by Kamino program
    pub farms_program: AccountInfo<'info>,

    /// CHECK: Verified by Kamino program
    #[account(mut)]
    pub kamino_reserve_farm_state: AccountInfo<'info>,
   
    pub collateral_token_program: Program<'info, Token>, 

    pub liquidity_token_program: Interface<'info, TokenInterface>, 

    pub system_program: Program<'info, System>,
    
    pub associated_token_program: Program<'info, AssociatedToken>,

    #[account(address = sysvar::instructions::ID)]
    /// CHECK: This is sysvar instructions account.
    pub instructions_sysvar: UncheckedAccount<'info>,

}
