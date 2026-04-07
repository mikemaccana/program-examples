use anchor_lang::prelude::*;

use crate::state::game;
use crate::CarnivalContext;

// Instruction Data

pub struct PlayGameInstructionData {
    pub gamer_name: String,
    pub gamer_ticket_count: u32,
    pub game: String,
}

pub fn handler(
    _ctx: Context<CarnivalContext>,
    name: String,
    ticket_count: u32,
    game_name: String,
) -> Result<()> {
    play_game(PlayGameInstructionData {
        gamer_name: name,
        gamer_ticket_count: ticket_count,
        game: game_name,
    })
}

pub fn play_game(ix: PlayGameInstructionData) -> Result<()> {
    let games_list = game::get_games();

    for game in games_list.iter() {
        if ix.game.eq(&game.name) {
            msg!("You're about to play {}!", game.name);

            if ix.gamer_ticket_count < game.tickets {
                msg!(
                    "  Sorry {}, you need {} tickets to play {}!",
                    ix.gamer_name,
                    game.tickets,
                    game.name
                );
            } else {
                msg!("  Let's see what you got!");
                msg!(
                    "  You get {} attempts and the prize is a {}!",
                    game.tries,
                    game.prize
                );
            };

            return Ok(());
        }
    }

    Err(ProgramError::InvalidInstructionData.into())
}
