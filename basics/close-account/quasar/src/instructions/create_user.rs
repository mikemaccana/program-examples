use {
    crate::state::UserState,
    quasar_lang::prelude::*,
};

/// Accounts for creating a new user.
#[derive(Accounts)]
pub struct CreateUser {
    #[account(mut)]
    pub user: Signer,
    #[account(mut, init, payer = user, seeds = UserState::seeds(user), bump)]
    pub user_account: Account<UserState<'_>>,
    pub system_program: Program<System>,
}

#[inline(always)]
pub fn handle_create_user(accounts: &mut CreateUser, name: &str, bump: u8) -> Result<(), ProgramError> {
    let user_address = *accounts.user.to_account_view().address();
    accounts.user_account.set_inner(
        bump,
        user_address,
        name,
        accounts.user.to_account_view(),
        None,
    )
}
