use {
    crate::state::Favorites,
    quasar_lang::prelude::*,
};

/// Accounts for setting user favourites. Uses `init_if_needed` so the same
/// instruction can create or update the favourites PDA.
#[derive(Accounts)]
pub struct SetFavorites {
    #[account(mut)]
    pub user: Signer,
    #[account(mut, init_if_needed, payer = user, seeds = Favorites::seeds(user), bump)]
    pub favorites: Account<Favorites<'_>>,
    pub system_program: Program<System>,
}

#[inline(always)]
pub fn handle_set_favorites(accounts: &mut SetFavorites, number: u64, color: &str) -> Result<(), ProgramError> {
    accounts.favorites.set_inner(
        number,
        color,
        accounts.user.to_account_view(),
        None,
    )
}
