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
use fixed::types::I80F48;
use crate::accounts_zerocopy::AccountInfoRef;
use crate::health::{new_fixed_order_account_retriever_with_optional_banks, new_health_cache_skipping_missing_banks_and_bad_oracles, HealthType};
use crate::logs::{emit_stack, DepositLog, TokenBalanceLog};
use crate::require_msg_typed;
use crate::util::{clock_now, sighash};
use crate::state::{oracle_state_unchecked, Bank, Market, OracleAccountInfos, PeachAccountFixed, PeachAccountLoader};
use crate::constants::KAMINO_PROGRAM_ID;
use crate::error::*;

struct DepositCommon<'a, 'info> {
    pub market: &'a AccountLoader<'info, Market>,
    pub account: &'a AccountLoader<'info, PeachAccountFixed>,
    pub bank: &'a AccountLoader<'info, Bank>,
    // pub vault: &'a Account<'info, TokenAccount>,
    pub oracle: &'a UncheckedAccount<'info>,
}

impl<'a, 'info> DepositCommon<'a, 'info> {
    fn deposit_into_existing(
        &self,
        remaining_accounts: &[AccountInfo],
        amount: u64,
        reduce_only: bool,
        allow_token_account_closure: bool,
    ) -> Result<()> {
        require!(amount > 0, PeachError::InvalidAmount);

        let mut bank = self.bank.load_mut()?;
        let token_index = bank.token_index;

        let amount_i80f48 = {
            // Get the account's position for that token index
            let account = self.account.load_full()?;
            let position = account.token_position(token_index)?;

            let amount_i80f48 = if reduce_only || bank.are_deposits_reduce_only() {
                position
                    .native(&bank)
                    .min(I80F48::ZERO)
                    .abs()
                    .ceil()
                    .min(I80F48::from(amount))
            } else {
                I80F48::from(amount)
            };
            if bank.are_deposits_reduce_only() {
                require!(
                    reduce_only || amount_i80f48 == I80F48::from(amount),
                    PeachError::TokenInReduceOnlyMode
                );
            }
            amount_i80f48
        };

        // Get the account's position for that token index
        let mut account = self.account.load_full_mut()?;

        let (position, raw_token_index) = account.token_position_mut(token_index)?;

        let position_is_active = {
            bank.deposit(
                position,
                amount_i80f48,
                Clock::get()?.unix_timestamp.try_into().unwrap(),
            )?
        };

        let indexed_position = position.indexed_position;

        // Get the oracle price, even if stale or unconfident: We want to allow users
        // to deposit to close borrows or do other fixes even if the oracle is bad.
        let oracle_ref = &AccountInfoRef::borrow(self.oracle.as_ref())?;
        let unsafe_oracle_state = oracle_state_unchecked(
            &OracleAccountInfos::from_reader(oracle_ref),
            bank.mint_decimals,
        )?;
        let unsafe_oracle_price = unsafe_oracle_state.price;

        // If increasing total deposits, check deposit limits
        if indexed_position.is_positive() {
            bank.check_deposit_and_oo_limit()?;
        }

        // Update the net deposits - adjust by price so different tokens are on the same basis (in USD terms)
        let amount_usd = (amount_i80f48 * unsafe_oracle_price).to_num::<i64>();
        account.fixed.net_deposits += amount_usd;

        emit_stack(TokenBalanceLog {
            peach_market: self.market.key(),
            peach_account: self.account.key(),
            token_index: token_index.0,
            indexed_position: indexed_position.0,
            deposit_index: bank.deposit_index.0,
            borrow_index: bank.borrow_index.0,
        });
        drop(bank);

        //
        // Health computation
        //
        let (now_ts, now_slot) = clock_now();
        let retriever = new_fixed_order_account_retriever_with_optional_banks(
            remaining_accounts,
            &account.borrow(),
            (now_ts, now_slot),
        )?;

        // We only compute health to check if the account leaves the being_liquidated state.
        // So it's ok to possibly skip nonnegative token positions and compute a health
        // value that is too low.
        let cache = new_health_cache_skipping_missing_banks_and_bad_oracles(
            &account.borrow(),
            &retriever,
            now_ts,
        )?;

        // Since depositing can only increase health, we can skip the usual pre-health computation.
        // Also, TokenDeposit is one of the rare instructions that is allowed even during being_liquidated.
        // Being in a health region always means being_liquidated is false, so it's safe to gate the check.
        let was_being_liquidated = account.being_liquidated();
        if !account.fixed.is_in_health_region() && was_being_liquidated {
            let health = cache.health(HealthType::LiquidationEnd);
            msg!("health: {}", health);
            // Only compute health and check for recovery if not already being liquidated

            let recovered = account.fixed.maybe_recover_from_being_liquidated(health);
            require!(recovered, PeachError::DepositsIntoLiquidatingMustRecover);
        }

        // Market level deposit limit on account
        let market = self.market.load()?;
        if market.deposit_limit_quote > 0 {
            // Requires that all banks were provided and all oracles are healthy, otherwise we
            // can't know how much this account has deposited
            require_eq!(
                cache.token_infos.len(),
                account.active_token_positions().count()
            );

            let assets = cache
                .health_assets_and_liabs_stable_assets(HealthType::Init)
                .0
                .round_to_zero()
                .to_num::<u64>();
            require_msg_typed!(
                assets <= market.deposit_limit_quote,
                PeachError::DepositLimit,
                "assets ({}) can't cross deposit limit on the market ({})",
                assets,
                market.deposit_limit_quote
            );
        }

        //
        // Deactivate the position only after the health check because the user passed in
        // remaining_accounts for all banks/oracles, including the account that will now be
        // deactivated.
        // Deposits can deactivate a position if they cancel out a previous borrow.
        //
        if allow_token_account_closure && !position_is_active {
            account.deactivate_token_position_and_log(raw_token_index, self.account.key());
        }

        emit_stack(DepositLog {
            peach_market: self.market.key(),
            peach_account: self.account.key(),
            signer: self.market.key(), // TODO: this should be the kamino cpi signer
            token_index: token_index.0,
            quantity: amount_i80f48.to_num::<u64>(),
            price: unsafe_oracle_price.to_bits(),
        });

        Ok(())
    }

}

pub fn kamino_deposit<'info>(
    ctx: Context<'_, '_, '_, 'info, DepositKamino<'info>>,
    deposit_amount: u64, 
    reduce_only: bool
    // protocol_index: u8,
) -> Result<()> {
    
    require!(deposit_amount > 0, PeachError::InvalidAmount);

    // msg!("Deposit amount: {:?}", deposit_amount);    
    // msg!("Deposit mint: {:?}", ctx.accounts.mint.key()); 

    // msg!("CPI Tranfer: Kamino - Deposit");

    // Creating/Updating Token Position in our Peach program
    {
        let token_index = ctx.accounts.bank.load()?.token_index;
        let mut account = ctx.accounts.peach_account.load_full_mut()?;

        let token_position_exists = account
            .all_token_positions()
            .any(|p| p.is_active_for_token(token_index));

        // Activating a new token position requires that the oracle is in a good state.
        // Otherwise users could abuse oracle staleness to delay liquidation.
        if !token_position_exists {
            let (now_ts, now_slot) =
                Clock::get().map(|c| (c.unix_timestamp as u64, c.slot as u64))?;
            let bank = ctx.accounts.bank.load()?;

            let oracle_ref = &AccountInfoRef::borrow(ctx.accounts.oracle.as_ref())?;
            let oracle_result = bank.oracle_price(
                &OracleAccountInfos::from_reader(oracle_ref),
                Some((now_ts, now_slot)),
            );
            if let Err(e) = oracle_result {
                msg!("oracle must be valid when creating a new token position");
                return Err(e);
            }

            account.ensure_token_position(token_index, 1)?;
        }

        // Update the token position with the new deposit amount
        DepositCommon {
            market: &ctx.accounts.market,
            account: &ctx.accounts.peach_account,
            bank: &ctx.accounts.bank,
            // vault: &ctx.accounts.vault,
            oracle: &ctx.accounts.oracle,
        }
        .deposit_into_existing(ctx.remaining_accounts, deposit_amount, reduce_only, true)?; // TODO : add reduce_only and allow_token_account_closure
    }

    // Kamino CPI
    let (kamino_reserve_liquidity_usdc_supply_pda, _bump) = Pubkey::find_program_address(
        &[
            b"reserve_liq_supply",
            ctx.accounts.lending_market.key().as_ref(),
            ctx.accounts.mint.key().as_ref(),
        ],
        &KAMINO_PROGRAM_ID,  // Kamino program ID here!
    );

    require_keys_eq!(
        kamino_reserve_liquidity_usdc_supply_pda,
        ctx.accounts.kamino_reserve_liquidity_usdc_supply.key(),
        PeachError::InvalidKaminoReserveLiquiditySupplyAccount
    );

    let accounts = vec![
        AccountMeta::new(ctx.accounts.signer.key(), true), 
        AccountMeta::new(ctx.accounts.obligation.key(), false), 
        AccountMeta::new_readonly(ctx.accounts.lending_market.key(), false), 
        AccountMeta::new_readonly(ctx.accounts.lending_market_authority.key(), false), 
        AccountMeta::new(ctx.accounts.kamino_reserve.key(), false),     
        AccountMeta::new_readonly(ctx.accounts.mint.key(), false),
        AccountMeta::new(ctx.accounts.kamino_reserve_liquidity_usdc_supply.key(), false),
        AccountMeta::new(ctx.accounts.kamino_collateral_mint.key(), false),
        AccountMeta::new(ctx.accounts.kamino_destination_deposit_collateral.key(), false),
        AccountMeta::new(ctx.accounts.user_token_account.key(), false),
        // AccountMeta::new_readonly(ctx.accounts.user_kamino_reserve_usdc_token_account.key(), false), //placeholder
        AccountMeta::new_readonly(ctx.accounts.kamino_program.key(), false), // placeholder used in klend-sdk
        AccountMeta::new_readonly(ctx.accounts.collateral_token_program.key(), false),
        AccountMeta::new_readonly(ctx.accounts.liquidity_token_program.key(), false),
        AccountMeta::new_readonly(ctx.accounts.instructions_sysvar.key(), false),
        // AccountMeta::new_readonly(ctx.accounts.kamino_program.key(), false), // not needed for kamino program
        AccountMeta::new(ctx.accounts.kamino_obligation_farm_user_state.key(), false),
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


    let _lending_hub_key = ctx.accounts.market.key();
    let _account_seeds = & ctx.accounts.peach_account.load()?.pda_seeds();

    
    invoke_signed(
        &kamino_deposit_ix,
        &[
            ctx.accounts.signer.to_account_info(),
            ctx.accounts.obligation.clone(),
            ctx.accounts.lending_market.clone(),
            ctx.accounts.lending_market_authority.clone(),
            ctx.accounts.kamino_reserve.clone(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.kamino_reserve_liquidity_usdc_supply.clone(),
            ctx.accounts.kamino_collateral_mint.to_account_info(),
            ctx.accounts.kamino_destination_deposit_collateral.clone(),
            ctx.accounts.user_token_account.to_account_info(),
            // ctx.accounts.user_kamino_reserve_usdc_token_account.to_account_info(), 
            ctx.accounts.kamino_program.clone(), 
            ctx.accounts.collateral_token_program.to_account_info(),
            ctx.accounts.liquidity_token_program.to_account_info(),
            ctx.accounts.instructions_sysvar.to_account_info(), 
            ctx.accounts.kamino_obligation_farm_user_state.clone(),
            ctx.accounts.kamino_reserve_farm_state.clone(),
            ctx.accounts.farms_program.clone(),
            // ctx.accounts.kamino_program.clone(),
        ],
        &[],
        // &[&account_seeds.signer_seeds()],
    )?;

    Ok(())
}


#[derive(Accounts)]
pub struct DepositKamino<'info> {

    #[account(
        mut,
        address = peach_account.load()?.owner,
    )]
    pub signer: Signer<'info>,

    /// CHECK: Verified by Kamino program
    #[account(mut)]
    pub obligation: AccountInfo<'info>,

    #[account(
        mut,
        has_one = market
    )]
    pub peach_account: AccountLoader<'info, PeachAccountFixed>,

    #[account(
        mut,
        has_one = market,
        // has_one = vault,
        has_one = oracle,
        // the mints of bank/vault/token_account are implicitly the same because
        // spl::token::transfer succeeds between token_account and vault
    )]
    pub bank: AccountLoader<'info, Bank>,

    // #[account(mut)]
    // pub vault: Account<'info, TokenAccount>,

    /// CHECK: The oracle can be one of several different account types
    pub oracle: UncheckedAccount<'info>,

    pub market: AccountLoader<'info, Market>,

    #[account(mut)]
    pub kamino_collateral_mint: InterfaceAccount<'info, Mint>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = kamino_collateral_mint,
        associated_token::authority = peach_account,
        associated_token::token_program = collateral_token_program,
    )]
    pub user_kamino_reserve_usdc_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = mint,
        token::authority = signer,
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


    #[account(address = KAMINO_PROGRAM_ID)]
    /// CHECK: Kamino program ID
    pub kamino_program: AccountInfo<'info>, 
    /// Kamino Farms program
    /// CHECK: Verified by Kamino program
    pub farms_program: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: Verified by Kamino program
    pub kamino_obligation_farm_user_state: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: Verified by Kamino program
    pub kamino_reserve_farm_state: AccountInfo<'info>,
   
    pub collateral_token_program: Program<'info, Token>, 

    pub liquidity_token_program: Interface<'info, TokenInterface>, 

    pub system_program: Program<'info, System>,
    
    pub associated_token_program: Program<'info, AssociatedToken>,

    #[account(address = sysvar::instructions::ID)]
    /// CHECK: This is sysvar instructions account.
    pub instructions_sysvar: UncheckedAccount<'info>,
}
