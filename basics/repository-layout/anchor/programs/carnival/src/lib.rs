use anchor_lang::prelude::*;

pub mod error;
pub mod instructions;
pub mod state;

use instructions::*;

// For setting up modules & configs

declare_id!("FY78H521JC85fwffNHeV5BjKvhvRVuVtFgVrick7JEQj");

#[program]
pub mod carnival {
    use super::*;

    pub fn go_on_ride(
        ctx: Context<CarnivalContext>,
        name: String,
        height: u32,
        ticket_count: u32,
        ride_name: String,
    ) -> Result<()> {
        get_on_ride::handler(ctx, name, height, ticket_count, ride_name)
    }

    pub fn play_game(
        ctx: Context<CarnivalContext>,
        name: String,
        ticket_count: u32,
        game_name: String,
    ) -> Result<()> {
        play_game::handler(ctx, name, ticket_count, game_name)
    }

    pub fn eat_food(
        ctx: Context<CarnivalContext>,
        name: String,
        ticket_count: u32,
        food_stand_name: String,
    ) -> Result<()> {
        eat_food::handler(ctx, name, ticket_count, food_stand_name)
    }
}

#[derive(Accounts)]
pub struct CarnivalContext<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
}
