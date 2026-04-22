use {
    crate::state::Escrow,
    quasar_lang::prelude::*,
    quasar_spl::{Mint, Token, TokenCpi},
};

#[derive(Accounts)]
pub struct Make {
    #[account(mut)]
    pub maker: Signer,
    #[account(mut, init, payer = maker, seeds = Escrow::seeds(maker), bump)]
    pub escrow: Account<Escrow>,
    pub mint_a: Account<Mint>,
    pub mint_b: Account<Mint>,
    #[account(mut)]
    pub maker_ta_a: Account<Token>,
    #[account(mut, init_if_needed, payer = maker, token::mint = mint_b, token::authority = maker)]
    pub maker_ta_b: Account<Token>,
    #[account(mut, init_if_needed, payer = maker, token::mint = mint_a, token::authority = escrow)]
    pub vault_ta_a: Account<Token>,
    pub rent: Sysvar<Rent>,
    pub token_program: Program<Token>,
    pub system_program: Program<System>,
}

#[inline(always)]
pub fn handle_make_escrow(accounts: &mut Make, receive: u64, bumps: &MakeBumps) -> Result<(), ProgramError> {
    accounts.escrow.set_inner(
        *accounts.maker.address(),
        *accounts.mint_a.address(),
        *accounts.mint_b.address(),
        *accounts.maker_ta_b.address(),
        receive,
        bumps.escrow,
    );
    Ok(())
}

#[inline(always)]
pub fn handle_deposit_tokens(accounts: &mut Make, amount: u64) -> Result<(), ProgramError> {
    accounts.token_program
        .transfer(&accounts.maker_ta_a, &accounts.vault_ta_a, &accounts.maker, amount)
        .invoke()
}
