use crate::accounts_zerocopy::AccountInfoRef;
use crate::constants::KAMINO_PROGRAM_ID;
use crate::error::*;
use crate::health::{
    new_fixed_order_account_retriever_with_optional_banks,
    new_health_cache_skipping_missing_banks_and_bad_oracles, HealthType,
};
use crate::logs::{emit_stack, DepositLog, TokenBalanceLog};
use crate::require_msg_typed;
use crate::state::{
    oracle_state_unchecked, Bank, IxGate, Market, OracleAccountInfos, PeachAccountFixed, PeachAccountLoader
};
use crate::util::{clock_now, sighash};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::sysvar;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};
use fixed::types::I80F48;

#[cfg(feature="mainnet")]
use anchor_lang::solana_program::program::invoke;

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

        msg!("Token index {}", token_index);

        let amount_i80f48 = {
            // Get the account's position for that token index
            let account = self.account.load_full()?;

            msg!("Current net deposit {}", account.fixed.net_deposits_kamino);

            let position = account.token_position_kamino(token_index)?;

            msg!("Position for token index {:?}", position);

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

        msg!("Repay amount {}", amount_i80f48);

        // Get the account's position for that token index
        let mut account = self.account.load_full_mut()?;

        let (position, raw_token_index) = account.token_position_mut_kamino(token_index)?;

        let position_is_active = {
            bank.deposit(
                position,
                amount_i80f48,
                Clock::get()?.unix_timestamp.try_into().unwrap(),
            )?
        };

        msg!("Position active status: {}", position_is_active);

        let indexed_position = position.indexed_position;

        // Get the oracle price, even if stale or unconfident: We want to allow users
        // to deposit to close borrows or do other fixes even if the oracle is bad.
        let oracle_ref = &AccountInfoRef::borrow(self.oracle.as_ref())?;
        let unsafe_oracle_state = oracle_state_unchecked(
            &OracleAccountInfos::from_reader(oracle_ref),
            bank.mint_decimals,
        )?;
        let unsafe_oracle_price = unsafe_oracle_state.price;

        msg!("Unsafe oracle price for token index {}", unsafe_oracle_price);

        // If increasing total deposits, check deposit limits
        if indexed_position.is_positive() {
            bank.check_deposit_and_oo_limit()?;
        }

        // Update the net deposits - adjust by price so different tokens are on the same basis (in USD terms)
        let amount_usd = (amount_i80f48 * unsafe_oracle_price).to_num::<i64>();
        account.fixed.net_deposits_kamino += amount_usd;

        msg!("Net deposits after repay: {}", account.fixed.net_deposits_kamino);

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
            msg!(
                "Deactivated token position for token index {}",
                token_index
            );
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

pub fn kamino_repay<'info>(
    ctx: Context<'_, '_, '_, 'info, RepayKamino<'info>>,
    deposit_amount: u64,
    reduce_only: bool, // protocol_index: u8,
) -> Result<()> {
    require!(deposit_amount > 0, PeachError::InvalidAmount);

    msg!("Repay amount: {:?}", deposit_amount);
    msg!("Repay mint: {:?}", ctx.accounts.reserve_liquidity_mint.key());

    // Creating/Updating Token Position in our Peach program
    {
        let token_index = ctx.accounts.bank.load()?.token_index;
        let mut account = ctx.accounts.peach_account.load_full_mut()?;

        let token_position_exists = account
            .all_token_positions()
            .any(|p| p.is_active_for_token(token_index) && p.is_kamino_position());


        msg!("Token position exists: {}", token_position_exists);

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

    msg!("CPI Tranfer: Kamino - Repay");

    let accounts = vec![
        AccountMeta::new(ctx.accounts.owner.key(), true), // owner
        AccountMeta::new(ctx.accounts.obligation.key(), false), // obligation
        AccountMeta::new_readonly(ctx.accounts.lending_market.key(), false), // lending_market
        AccountMeta::new(ctx.accounts.repay_reserve.key(), false), // repay_reserve
        AccountMeta::new_readonly(ctx.accounts.reserve_liquidity_mint.key(), false), // reserve_liquidity_mint
        AccountMeta::new(ctx.accounts.reserve_destination_liquidity.key(), false), // reserve_destination_liquidity
        AccountMeta::new(ctx.accounts.user_source_liquidity.key(), false), // user_source_liquidity
        AccountMeta::new_readonly(ctx.accounts.token_program.key(), false), // token_program
        AccountMeta::new_readonly(ctx.accounts.instruction_sysvar_account.key(), false), // instruction_sysvar_account
        AccountMeta::new(ctx.accounts.obligation_farm_user_state.key(), false), // obligation_farm_user_state
        AccountMeta::new(ctx.accounts.reserve_farm_state.key(), false), // reserve_farm_state
        AccountMeta::new_readonly(ctx.accounts.lending_market_authority.key(), false), // lending_market_authority
        AccountMeta::new_readonly(ctx.accounts.farms_program.key(), false),
    ];

    let discriminator = sighash(
        "global",
        "repay_obligation_liquidity_v2",
    );

    let mut data = discriminator.to_vec();
    data.extend_from_slice(&deposit_amount.to_le_bytes());

    let kamino_repay_ix = Instruction {
        program_id: KAMINO_PROGRAM_ID,
        accounts,
        data,
    };

    let account_infos = &[
        ctx.accounts.owner.to_account_info(),
        ctx.accounts.obligation.clone(),
        ctx.accounts.lending_market.clone(),
        ctx.accounts.repay_reserve.clone(),
        ctx.accounts.reserve_liquidity_mint.to_account_info(),
        ctx.accounts.reserve_destination_liquidity.to_account_info(),
        ctx.accounts.user_source_liquidity.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.instruction_sysvar_account.to_account_info(),
        ctx.accounts.obligation_farm_user_state.clone(),
        ctx.accounts.reserve_farm_state.clone(),
        ctx.accounts.lending_market_authority.clone(),
        ctx.accounts.farms_program.clone(),
        ctx.accounts.kamino_program.clone(),
    ];

    #[cfg(feature = "mainnet")]
    {
        invoke(&kamino_repay_ix, account_infos)?;
    }

    #[cfg(not(feature = "mainnet"))]
    {
        msg!("skipped CPI invoke for testnet/devnet");
    }

    Ok(())
}

#[derive(Accounts)]
pub struct RepayKamino<'info> {
    #[account(
        constraint = market.load()?.is_ix_enabled(IxGate::TokenDeposit) @ PeachError::IxIsDisabled,
    )]
    pub market: AccountLoader<'info, Market>,
    
    #[account(
        mut,
        has_one = market,
        // constraint = peach_account.load()?.is_operational() @ PeachError::AccountIsFrozen,
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

    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: Verified by Kamino program
    #[account(mut)]
    pub obligation: AccountInfo<'info>,

    /// CHECK: Verified by Kamino program
    pub lending_market: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: Verified by Kamino program
    pub repay_reserve: AccountInfo<'info>,

    pub reserve_liquidity_mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    /// CHECK: Verified by Kamino program / process method
    pub reserve_destination_liquidity: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = reserve_liquidity_mint,
        token::authority = owner,
    )] 
    pub user_source_liquidity: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,

    #[account(address = sysvar::instructions::ID)]
    /// CHECK: This is sysvar instructions account.
    pub instruction_sysvar_account: UncheckedAccount<'info>,
    
    #[account(mut)]
    /// CHECK: Verified by Kamino program
    pub obligation_farm_user_state: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: Verified by Kamino program
    pub reserve_farm_state: AccountInfo<'info>,

    /// CHECK: Verified via PDA constraints
    pub lending_market_authority: AccountInfo<'info>,

    /// Kamino Farms program
    /// CHECK: Verified by Kamino program
    pub farms_program: AccountInfo<'info>,

    #[account(address = KAMINO_PROGRAM_ID)]
    /// CHECK: Kamino program ID
    pub kamino_program: AccountInfo<'info>,
    
    pub system_program: Program<'info, System>,

    pub associated_token_program: Program<'info, AssociatedToken>,
}
