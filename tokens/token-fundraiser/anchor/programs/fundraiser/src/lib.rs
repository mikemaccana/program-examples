use anchor_lang::prelude::*;

declare_id!("EuqudshRf8VRskatC6U3txgivHmtCkY5d1qsK5B3VAio");

mod constants;
mod error;
mod instructions;
mod state;

pub use constants::*;
use error::*;
use instructions::*;

#[program]
pub mod fundraiser {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, amount: u64, duration: u16) -> Result<()> {
        initialize::handler(ctx, amount, duration)
    }

    pub fn contribute(ctx: Context<Contribute>, amount: u64) -> Result<()> {
        contribute::handler(ctx, amount)
    }

    pub fn check_contributions(ctx: Context<CheckContributions>) -> Result<()> {
        checker::handler(ctx)
    }

    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        refund::handler(ctx)
    }
}
