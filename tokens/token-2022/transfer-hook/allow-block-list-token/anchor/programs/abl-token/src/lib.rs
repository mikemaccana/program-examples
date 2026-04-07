use anchor_lang::prelude::*;
use spl_discriminator::SplDiscriminate;
use spl_transfer_hook_interface::instruction::ExecuteInstruction;

pub mod constants;
pub mod errors;
pub mod instructions;
pub mod state;
pub mod utils;
pub use constants::*;
pub use errors::*;
pub use instructions::*;
pub use state::*;
pub use utils::*;

declare_id!("9kSUqBeRgchrr9HpGneGHxqZ19qLTYzRyYYm2qQgtDmU");

#[program]
pub mod abl_token {

    use super::*;

    pub fn init_mint(ctx: Context<InitMint>, args: InitMintArgs) -> Result<()> {
        init_mint::handler(ctx, args)
    }

    pub fn init_config(ctx: Context<InitConfig>) -> Result<()> {
        init_config::handler(ctx)
    }

    pub fn attach_to_mint(ctx: Context<AttachToMint>) -> Result<()> {
        attach_to_mint::handler(ctx)
    }

    #[instruction(discriminator = ExecuteInstruction::SPL_DISCRIMINATOR_SLICE)]
    pub fn tx_hook(ctx: Context<TxHook>, amount: u64) -> Result<()> {
        tx_hook::handler(ctx, amount)
    }

    pub fn init_wallet(ctx: Context<InitWallet>, args: InitWalletArgs) -> Result<()> {
        init_wallet::handler(ctx, args)
    }

    pub fn remove_wallet(ctx: Context<RemoveWallet>) -> Result<()> {
        remove_wallet::handler(ctx)
    }

    pub fn change_mode(ctx: Context<ChangeMode>, args: ChangeModeArgs) -> Result<()> {
        change_mode::handler(ctx, args)
    }
}
