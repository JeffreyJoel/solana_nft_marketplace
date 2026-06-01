use anchor_lang::prelude::*;
use mpl_core::{instructions::TransferV1CpiBuilder, ID as MPL_CORE_ID};

use crate::state::Listing;

#[derive(Accounts)]
pub struct ListAcoounts<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    /// CHECK: This is the maker Account
    #[account(mut)]
    pub asset: UncheckedAccount<'info>,

    /// CHECK: This is the collection Account
    #[account(mut)]
    pub collection: Option<UncheckedAccount<'info>>,

    #[account(
        init,
        payer = maker,
        seeds = [b"listing", asset.key().as_ref()],
        space = Listing::INIT_SPACE + Listing::DISCRIMINATOR.len(),
        bump
    )]
    pub listing: Account<'info, Listing>,

    /// CHECK: This is the Metaplex Core program
    #[account(address = MPL_CORE_ID)]
    pub mpl_core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> ListAcoounts<'info> {
    pub fn create_list(&mut self, price: u64, bumps: ListAcoountsBumps) -> Result<()> {
        self.listing.set_inner(Listing {
            maker: self.maker.key(),
            asset: self.asset.key(),
            price,
            bump: bumps.listing,
        });

        TransferV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(self.collection.as_ref().map(|c| c.as_ref()))
            .payer(&self.maker.to_account_info())
            .authority(Some(&self.maker.to_account_info()))
            .new_owner(&self.listing.to_account_info())
            .system_program(Some(&self.system_program.to_account_info()))
            .invoke()?;

        Ok(())
    }
}
