use anchor_lang::prelude::*;

mod constants;
mod errors;
pub mod instructions;
mod state;

use instructions::*;

// Set the correct key here
declare_id!("9FX4TuhmPbKEavcP431ALEzrvsrEaUNogPYcMZL37CeS");

#[program]
pub mod swap_example {
    use super::*;

    pub fn create_amm(ctx: Context<CreateAmm>, id: Pubkey, fee: u16) -> Result<()> {
        create_amm::handler(ctx, id, fee)
    }

    pub fn create_pool(ctx: Context<CreatePool>) -> Result<()> {
        create_pool::handler(ctx)
    }

    pub fn deposit_liquidity(
        ctx: Context<DepositLiquidity>,
        amount_a: u64,
        amount_b: u64,
    ) -> Result<()> {
        deposit_liquidity::handler(ctx, amount_a, amount_b)
    }

    pub fn withdraw_liquidity(ctx: Context<WithdrawLiquidity>, amount: u64) -> Result<()> {
        withdraw_liquidity::handler(ctx, amount)
    }

    pub fn swap_exact_tokens_for_tokens(
        ctx: Context<SwapExactTokensForTokens>,
        swap_a: bool,
        input_amount: u64,
        min_output_amount: u64,
    ) -> Result<()> {
        swap_exact_tokens_for_tokens::handler(ctx, swap_a, input_amount, min_output_amount)
    }
}
