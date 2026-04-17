use quasar_lang::prelude::*;

/// Onchain power status: a single boolean toggle.
#[account(discriminator = 1)]
pub struct PowerStatus {
    pub is_on: PodBool,
}
