use anchor_lang::prelude::*;

declare_id!("ATSXNT29EextctdVyhULT3kaHixp7eNbmwy5gH5pttwr");

#[program]
pub mod peach_v1 {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
