use crate::{error::PeachError, state::*};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct IxGateSet<'info> {
    #[account(
        mut,
        // market <-> admin relation is checked at #1
    )]
    pub market: AccountLoader<'info, Market>,
    pub admin: Signer<'info>,
}

pub fn ix_gate_set(ctx: Context<IxGateSet>, ix_gate: u128) -> Result<()> {
    let mut market = ctx.accounts.market.load_mut()?;

    msg!("old  {:?}, new {:?}", market.ix_gate, ix_gate);

    let mut require_market_admin = false;
    for i in 0..128 {
        // only admin can re-enable
        if market.ix_gate & (1 << i) != 0 && ix_gate & (1 << i) == 0 {
            require_market_admin = true;
        }
    }

    log_if_changed(&market, ix_gate, IxGate::AccountClose);
    log_if_changed(&market, ix_gate, IxGate::AccountCreate);
    log_if_changed(&market, ix_gate, IxGate::AccountEdit);
    log_if_changed(&market, ix_gate, IxGate::AccountExpand);
    log_if_changed(&market, ix_gate, IxGate::AccountToggleFreeze);
    log_if_changed(&market, ix_gate, IxGate::MarketClose);
    log_if_changed(&market, ix_gate, IxGate::MarketCreate);
    log_if_changed(&market, ix_gate, IxGate::StubOracleClose);
    log_if_changed(&market, ix_gate, IxGate::StubOracleCreate);
    log_if_changed(&market, ix_gate, IxGate::StubOracleSet);
    log_if_changed(&market, ix_gate, IxGate::TokenAddBank);
    log_if_changed(&market, ix_gate, IxGate::TokenDeposit);
    log_if_changed(&market, ix_gate, IxGate::TokenDeregister);
    log_if_changed(&market, ix_gate, IxGate::TokenRegister);
    log_if_changed(&market, ix_gate, IxGate::TokenRegisterTrustless);
    log_if_changed(&market, ix_gate, IxGate::TokenUpdateIndexAndRate);
    log_if_changed(&market, ix_gate, IxGate::TokenWithdraw);
    log_if_changed(&market, ix_gate, IxGate::AdminTokenWithdrawFees);
    log_if_changed(&market, ix_gate, IxGate::AccountSizeMigration);
    log_if_changed(&market, ix_gate, IxGate::TokenForceWithdraw);
    log_if_changed(&market, ix_gate, IxGate::TokenEdit);
    log_if_changed(&market, ix_gate, IxGate::MarketEdit);
    log_if_changed(&market, ix_gate, IxGate::TokenAddBank);

    market.ix_gate = ix_gate;

    // account constraint #1
    // Anam-Notes : Does not makes sense to have an if-else where if returns an error
    if require_market_admin {
        require!(
            market.admin == ctx.accounts.admin.key(),
            PeachError::SomeError
        );
    }
    Ok(())
}

fn log_if_changed(market: &Market, ix_gate: u128, ix: IxGate) {
    let old = market.is_ix_enabled(ix);
    let new = ix_gate & (1 << ix as u128) == 0;
    if old != new {
        msg!(
            "{:?} ix old {}, new {}",
            ix,
            if old { "enabled" } else { "disabled" },
            if new { "enabled" } else { "disabled" }
        );
    }
}
