use anchor_lang::prelude::*;

use instructions::*;
pub mod instructions;

declare_id!("DY2T6zhjngLvpkCMReQFzMHc6g4d4bqQWznntroEVDhG");

#[program]
pub mod metadata {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, args: TokenMetadataArgs) -> Result<()> {
        initialize::handler(ctx, args)
    }

    pub fn update_field(ctx: Context<UpdateField>, args: UpdateFieldArgs) -> Result<()> {
        update_field::handler(ctx, args)
    }

    pub fn remove_key(ctx: Context<RemoveKey>, key: String) -> Result<()> {
        remove_key::handler(ctx, key)
    }

    pub fn emit(ctx: Context<Emit>) -> Result<()> {
        emit::handler(ctx)
    }

    pub fn update_authority(ctx: Context<UpdateAuthority>) -> Result<()> {
        update_authority::handler(ctx)
    }
}
