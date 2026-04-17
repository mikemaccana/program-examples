use anchor_lang::prelude::*;

mod instructions;
use instructions::*;

declare_id!("Fod47xKXjdHVQDzkFPBvfdWLm8gEAV4iMSXkfUzCHiSD");

#[program]
pub mod anchor_realloc {
    use super::*;

    pub fn initialize(context: Context<Initialize>, input: String) -> Result<()> {
        instructions::initialize::handler(context, input)
    }

    pub fn update(context: Context<Update>, input: String) -> Result<()> {
        instructions::update::handler(context, input)
    }
}

#[account]
pub struct Message {
    pub message: String,
}

impl Message {
    pub fn required_space(input_len: usize) -> usize {
        8 + // 8 byte discriminator
        4 + // 4 byte for length of string
        input_len
    }
}
