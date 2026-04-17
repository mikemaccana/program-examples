use anchor_lang::prelude::*;

mod instructions;
use instructions::*;

declare_id!("BmDHboaj1kBUoinJKKSRqKfMeRKJqQqEbUj1VgzeQe4A");

#[program]
pub mod counter_anchor {
    use super::*;

    pub fn initialize_counter(context: Context<InitializeCounter>) -> Result<()> {
        instructions::initialize_counter::handler(context)
    }

    pub fn increment(context: Context<Increment>) -> Result<()> {
        instructions::increment::handler(context)
    }
}

#[account]
#[derive(InitSpace)]
pub struct Counter {
    count: u64,
}
