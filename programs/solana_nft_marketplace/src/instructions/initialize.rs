use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface};

use crate::{error::ErrorCode, state::MarketPlace};

#[derive(Accounts)]
#[instruction(name: String)]
pub struct InitializeAccount<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        seeds = [b"maketplace", name.as_bytes()],
        space = MarketPlace::INIT_SPACE + MarketPlace::DISCRIMINATOR.len(),
        bump
    )]
    pub maketplace: Account<'info, MarketPlace>,

    #[account(
        seeds = [b"treasury", maketplace.key().as_ref()],
        bump
    )]
    pub treasury: SystemAccount<'info>,

    #[account(
        init,
        payer = admin,
        seeds = [b"reward_mint", maketplace.key().as_ref()],
        bump,
        mint::decimals = 6,
        mint::authority = maketplace
    )]
    pub reward_mint: InterfaceAccount<'info, Mint>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> InitializeAccount<'info> {
    pub fn initialize(
        &mut self,
        name: String,
        fee: u16,
        bumps: InitializeAccountBumps,
    ) -> Result<()> {
        require!(fee <= 10_000, ErrorCode::InvalidFee);
        require!(name.len() <= 32, ErrorCode::InvalidName);

        self.maketplace.set_inner(MarketPlace {
            admin: self.admin.key(),
            fee,
            bump: bumps.maketplace,
            treasury_bump: bumps.treasury,
            reward_bump: bumps.reward_mint,
            name,
        });
        Ok(())
    }
}
