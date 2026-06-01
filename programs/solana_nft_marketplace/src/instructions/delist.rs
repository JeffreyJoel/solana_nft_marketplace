use anchor_lang::prelude::*;
use mpl_core::{instructions::TransferV1CpiBuilder, ID as MPL_CORE_ID};

use crate::state::{Listing, MarketPlace};

#[derive(Accounts)]
pub struct DelistAccounts<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,
    /// CHECK: This is the maker Account
    #[account(mut)]
    pub asset: UncheckedAccount<'info>,

    /// CHECK: This is the collection Account
    #[account(mut)]
    pub collection: Option<UncheckedAccount<'info>>,

    #[account(
        seeds = [b"maketplace", maketplace.name.as_bytes()],
        bump = maketplace.bump
    )]
    pub maketplace: Account<'info, MarketPlace>,

    #[account(
        mut,
        close = maker,
        seeds = [b"listing", listing.asset.as_ref()],
        bump = listing.bump,
        has_one = maker,
        has_one = asset,
    )]
    pub listing: Account<'info, Listing>,

    /// CHECK: This is the Metaplex Core program
    #[account(address = MPL_CORE_ID)]
    pub mpl_core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> DelistAccounts<'info> {
    pub fn refund(&mut self) -> Result<()> {
        let asset_key = self.asset.key();

        let bump = self.listing.bump;

        let seed = &[b"listing", asset_key.as_ref(), &[bump]];
        let signers_seeds: &[&[&[u8]]] = &[seed];

        TransferV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(self.collection.as_ref().map(|c| c.as_ref()))
            .payer(&self.maker.to_account_info())
            .authority(Some(&self.listing.to_account_info()))
            .new_owner(&self.maker.to_account_info())
            .system_program(Some(&self.system_program.to_account_info()))
            .invoke_signed(signers_seeds)?;

        Ok(())
    }
}
