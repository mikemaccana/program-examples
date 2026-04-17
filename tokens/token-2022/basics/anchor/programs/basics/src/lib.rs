use anchor_lang::prelude::*;

mod instructions;
use instructions::*;

declare_id!("6qNqxkRF791FXFeQwqYQLEzAbGiqDULC5SSHVsfRoG89");

#[program]
pub mod anchor {

    use super::*;

    pub fn create_token(context: Context<CreateToken>, token_name: String) -> Result<()> {
        instructions::create_token::handler(context, token_name)
    }

    pub fn create_token_account(context: Context<CreateTokenAccount>) -> Result<()> {
        instructions::create_token_account::handler(context)
    }

    pub fn create_associated_token_account(
        context: Context<CreateAssociatedTokenAccount>,
    ) -> Result<()> {
        instructions::create_associated_token_account::handler(context)
    }

    pub fn transfer_token(context: Context<TransferToken>, amount: u64) -> Result<()> {
        instructions::transfer_token::handler(context, amount)
    }

    pub fn mint_token(context: Context<MintToken>, amount: u64) -> Result<()> {
        instructions::mint_token::handler(context, amount)
    }
}
