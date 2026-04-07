use anchor_lang::prelude::*;
use instructions::*;
pub mod instructions;

declare_id!("EC9R1o4AsK3hoeh61yNyr2xiDoorgyS4ZBmhYujNUsyN");

#[program]
pub mod pda_rent_payer {
    use super::*;

    pub fn init_rent_vault(ctx: Context<InitRentVault>, fund_lamports: u64) -> Result<()> {
        init_rent_vault::handler(ctx, fund_lamports)
    }

    pub fn create_new_account(ctx: Context<CreateNewAccount>) -> Result<()> {
        create_new_account::handler(ctx)
    }
}
