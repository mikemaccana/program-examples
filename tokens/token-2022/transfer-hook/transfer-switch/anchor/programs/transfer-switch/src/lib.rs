mod error;
mod instructions;
mod state;

use anchor_lang::prelude::*;
use instructions::*;
use spl_discriminator::SplDiscriminate;
use spl_transfer_hook_interface::instruction::{
    ExecuteInstruction, InitializeExtraAccountMetaListInstruction,
};

declare_id!("H4LiorjbDr33X1KkTkWX9Eqy345uQH5HGNsxcPoXfGCg");

#[program]
pub mod transfer_switch {
    use super::*;

    pub fn configure_admin(ctx: Context<ConfigureAdmin>) -> Result<()> {
        configure_admin::handler(ctx)
    }

    #[instruction(discriminator = InitializeExtraAccountMetaListInstruction::SPL_DISCRIMINATOR_SLICE)]
    pub fn initialize_extra_account_metas_list(
        ctx: Context<InitializeExtraAccountMetas>,
    ) -> Result<()> {
        initialise_extra_account_metas_list::handler(ctx)
    }

    pub fn switch(ctx: Context<Switch>, on: bool) -> Result<()> {
        switch::handler(ctx, on)
    }

    #[instruction(discriminator = ExecuteInstruction::SPL_DISCRIMINATOR_SLICE)]
    pub fn transfer_hook(ctx: Context<TransferHook>, _amount: u64) -> Result<()> {
        transfer_hook::handler(ctx, _amount)
    }
}
