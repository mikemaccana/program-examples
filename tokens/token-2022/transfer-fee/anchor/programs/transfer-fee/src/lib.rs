use anchor_lang::prelude::*;

mod instructions;
use instructions::*;

declare_id!("FRB5Ln5fbH4sA1dyJRZz9M5mXQij8Tfr4cKtnP3gNaC1");

#[program]
pub mod transfer_fee {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        transfer_fee_basis_points: u16,
        maximum_fee: u64,
    ) -> Result<()> {
        initialize::handler(ctx, transfer_fee_basis_points, maximum_fee)
    }

    pub fn transfer(ctx: Context<Transfer>, amount: u64) -> Result<()> {
        transfer::handler(ctx, amount)
    }

    pub fn harvest<'info>(ctx: Context<'info, Harvest<'info>>) -> Result<()> {
        harvest::handler(ctx)
    }

    pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
        withdraw::handler(ctx)
    }

    pub fn update_fee(
        ctx: Context<UpdateFee>,
        transfer_fee_basis_points: u16,
        maximum_fee: u64,
    ) -> Result<()> {
        update_fee::handler(ctx, transfer_fee_basis_points, maximum_fee)
    }
}
