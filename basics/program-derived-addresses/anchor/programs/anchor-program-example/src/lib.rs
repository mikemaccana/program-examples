use anchor_lang::prelude::*;

use instructions::*;

pub mod instructions;
pub mod state;

declare_id!("GHC7zYxZ3mDfb9EunrPwbVd2FG8QpYbnVH3TEp9wBCCU");

#[program]
pub mod program_derived_addresses_program {
    use super::*;

    pub fn create_page_visits(ctx: Context<CreatePageVisits>) -> Result<()> {
        create::handler(ctx)
    }

    pub fn increment_page_visits(ctx: Context<IncrementPageVisits>) -> Result<()> {
        increment::handler(ctx)
    }
}
