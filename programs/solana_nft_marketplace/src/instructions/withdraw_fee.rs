use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

use crate::{error::ErrorCode, state::MarketPlace};

#[derive(Accounts)]
pub struct WithdrawFeeAccounts<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        has_one = admin,
        seeds = [b"maketplace", maketplace.name.as_bytes()],
        bump = maketplace.bump
    )]
    pub maketplace: Account<'info, MarketPlace>,

    #[account(
        seeds = [b"treasury", maketplace.key().as_ref()],
        bump = maketplace.treasury_bump
    )]
    pub treasury: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> WithdrawFeeAccounts<'info> {
    pub fn withdraw_fee(&mut self, amount: u64) -> Result<()> {
        require!(amount <= self.treasury.lamports(), ErrorCode::AmountTooMuch);
        let marketplace_key = self.maketplace.key();
        let seeds = &[
            b"treasury",
            marketplace_key.as_ref(),
            &[self.maketplace.treasury_bump],
        ];
        let signer_seeds: &[&[&[u8]]] = &[seeds];

        transfer(
            CpiContext::new_with_signer(
                self.system_program.to_account_info(),
                Transfer {
                    from: self.treasury.to_account_info(),
                    to: self.admin.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )
    }
}
