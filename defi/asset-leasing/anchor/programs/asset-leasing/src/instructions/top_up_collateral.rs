use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::{
    constants::{COLLATERAL_VAULT_SEED, LEASE_SEED},
    errors::AssetLeasingError,
    instructions::shared::transfer_tokens_from_user,
    state::{Lease, LeaseStatus},
};

#[derive(Accounts)]
pub struct TopUpCollateral<'info> {
    #[account(mut)]
    pub lessee: Signer<'info>,

    /// CHECK: program-derived address seed reference; no reads.
    pub lessor: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [LEASE_SEED, lessor.key().as_ref(), &lease.lease_id.to_le_bytes()],
        bump = lease.bump,
        has_one = lessor,
        has_one = collateral_mint,
        constraint = lease.lessee == lessee.key() @ AssetLeasingError::Unauthorised,
        constraint = lease.status == LeaseStatus::Active @ AssetLeasingError::InvalidLeaseStatus,
    )]
    pub lease: Account<'info, Lease>,

    pub collateral_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        seeds = [COLLATERAL_VAULT_SEED, lease.key().as_ref()],
        bump = lease.collateral_vault_bump,
        token::mint = collateral_mint,
        token::authority = collateral_vault,
        token::token_program = token_program,
    )]
    pub collateral_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = collateral_mint,
        associated_token::authority = lessee,
        associated_token::token_program = token_program,
    )]
    pub lessee_collateral_account: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handle_top_up_collateral(context: Context<TopUpCollateral>, amount: u64) -> Result<()> {
    require!(amount > 0, AssetLeasingError::InvalidCollateralAmount);

    transfer_tokens_from_user(
        &context.accounts.lessee_collateral_account,
        &context.accounts.collateral_vault,
        amount,
        &context.accounts.collateral_mint,
        &context.accounts.lessee,
        &context.accounts.token_program,
    )?;

    context.accounts.lease.collateral_amount = context
        .accounts
        .lease
        .collateral_amount
        .checked_add(amount)
        .ok_or(AssetLeasingError::MathOverflow)?;

    Ok(())
}
