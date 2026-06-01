use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

use crate::state::{Listing, Offer};

#[derive(Accounts)]
pub struct MakeOfferAccounts<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        init,
        payer = maker,
        seeds = [b"offer", listing.key().as_ref(), maker.key().as_ref()],
        space = Offer::INIT_SPACE + Offer::DISCRIMINATOR.len(),
        bump
    )]
    pub offer: Account<'info, Offer>,

    #[account(
        mut,
        seeds = [b"offer_vault", offer.key().as_ref()],
        bump
    )]
    pub offer_vault: SystemAccount<'info>,
    #[account(
        mut,
        seeds = [b"listing", listing.asset.as_ref()],
        bump = listing.bump,
    )]
    pub listing: Account<'info, Listing>,

    pub system_program: Program<'info, System>,
}

impl<'info> MakeOfferAccounts<'info> {
    pub fn make_offer(&mut self, price: u64, bumps: MakeOfferAccountsBumps) -> Result<()> {
        transfer(
            CpiContext::new(
                self.system_program.to_account_info(),
                Transfer {
                    from: self.maker.to_account_info(),
                    to: self.offer_vault.to_account_info(),
                },
            ),
            price,
        )?;

        self.offer.set_inner(Offer {
            listing: self.listing.key(),
            offer_maker: self.maker.key(),
            price,
            bump: bumps.offer,
            vault_bump: bumps.offer_vault,
        });
        Ok(())
    }
}
