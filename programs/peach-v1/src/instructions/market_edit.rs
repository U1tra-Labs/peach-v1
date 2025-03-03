use anchor_lang::prelude::*;

use crate::state::Market;

#[allow(clippy::too_many_arguments)]
pub fn market_edit(
    ctx: Context<MarketEdit>,
    admin_opt: Option<Pubkey>,
    testing_opt: Option<u8>,
    version_opt: Option<u8>,
    deposit_limit_quote_opt: Option<u64>,
    collateral_fee_interval_opt: Option<u64>,
) -> Result<()> {
    let mut market = ctx.accounts.market.load_mut()?;

    if let Some(admin) = admin_opt {
        require_keys_neq!(admin, Pubkey::default());
        msg!("Admin old {:?}, new {:?}", market.admin, admin);
        market.admin = admin;
    }

    if let Some(testing) = testing_opt {
        msg!("Testing old {:?}, new {:?}", market.testing, testing);
        market.testing = testing;
    }

    if let Some(version) = version_opt {
        msg!("Version old {:?}, new {:?}", market.version, version);
        market.version = version;
    }

    if let Some(deposit_limit_quote) = deposit_limit_quote_opt {
        msg!(
            "Deposit limit quote old {:?}, new {:?}",
            market.deposit_limit_quote,
            deposit_limit_quote
        );
        market.deposit_limit_quote = deposit_limit_quote;
    }

    if let Some(collateral_fee_interval) = collateral_fee_interval_opt {
        msg!(
            "Collateral fee interval old {:?}, new {:?}",
            market.collateral_fee_interval,
            collateral_fee_interval
        );
        market.collateral_fee_interval = collateral_fee_interval;
    }

    Ok(())
}

#[derive(Accounts)]
pub struct MarketEdit<'info> {
    #[account(
        mut,
        has_one = admin,
    )]
    pub market: AccountLoader<'info, Market>,
    pub admin: Signer<'info>,
}
