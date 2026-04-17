use quasar_lang::prelude::*;

/// Onchain counter account.
#[account(discriminator = 1)]
pub struct Counter {
    pub count: u64,
}
