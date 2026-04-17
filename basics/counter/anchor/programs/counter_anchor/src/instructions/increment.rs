use anchor_lang::prelude::*;

use crate::Counter;

#[derive(Accounts)]
pub struct Increment<'info> {
    #[account(mut)]
    pub counter: Account<'info, Counter>,
}

pub fn handler(context: Context<Increment>) -> Result<()> {
    context.accounts.counter.count = context.accounts.counter.count.checked_add(1).unwrap();
    Ok(())
}
