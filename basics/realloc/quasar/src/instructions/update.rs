use {
    crate::state::MessageAccount,
    quasar_lang::prelude::*,
};

/// Accounts for updating a message account.
/// Quasar's `set_inner` automatically handles realloc when the new message
/// is longer than the current account data. No explicit realloc needed.
#[derive(Accounts)]
pub struct Update {
    #[account(mut)]
    pub payer: Signer,
    #[account(mut)]
    pub message_account: Account<MessageAccount<'_>>,
    pub system_program: Program<System>,
}

#[inline(always)]
pub fn handle_update(accounts: &mut Update, message: &str) -> Result<(), ProgramError> {
    accounts.message_account.set_inner(
        message,
        accounts.payer.to_account_view(),
        None,
    )
}
