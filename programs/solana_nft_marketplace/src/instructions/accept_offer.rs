use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{mint_to_checked, Mint, MintToChecked, TokenAccount, TokenInterface},
};
use mpl_core::{instructions::TransferV1CpiBuilder, ID as MPL_CORE_ID};

use crate::state::{Listing, MarketPlace, Offer};

#[derive(Accounts)]
pub struct AcceptOfferAccounts<'info> {
    /// CHECK:
    #[account(mut)]
    pub offer_maker: UncheckedAccount<'info>,

    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        mut,
        close = offer_maker,
        seeds = [b"offer", offer.listing.as_ref(), offer.offer_maker.as_ref()],
        bump = offer.bump,
        has_one = offer_maker,
        has_one = listing,
    )]
    pub offer: Account<'info, Offer>,

    #[account(
        mut,
        seeds = [b"offer_vault", offer.key().as_ref()],
        bump = offer.vault_bump
    )]
    pub offer_vault: SystemAccount<'info>,

    /// CHECK: This is the asset Account
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

    #[account(
        mut,
        seeds = [b"treasury", maketplace.key().as_ref()],
        bump = maketplace.treasury_bump
    )]
    pub treasury: SystemAccount<'info>,

    #[account(
        seeds = [b"reward_mint", maketplace.key().as_ref()],
        bump = maketplace.treasury_bump,
        mint::decimals = 6,
        mint::authority = maketplace
    )]
    pub reward_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed,
        payer = offer_maker,
        associated_token::mint = reward_mint,
        associated_token::authority = offer_maker,
        associated_token::token_program = token_program
    )]
    pub take_reward_ata: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: This is the Metaplex Core program
    #[account(address = MPL_CORE_ID)]
    pub mpl_core_program: UncheckedAccount<'info>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,

    pub system_program: Program<'info, System>,
}

impl<'info> AcceptOfferAccounts<'info> {
    pub fn send_sol(&mut self) -> Result<()> {
        let price = self.offer_vault.lamports();
        let fee = (price as u128)
            .checked_mul(self.maketplace.fee as u128)
            .unwrap()
            .checked_div(10_000)
            .unwrap() as u64;

        let maker_amount = price.checked_sub(fee).unwrap();

        let offer_key = self.offer.key();
        let signers_seeds: &[&[&[u8]]] =
            &[&[b"offer_vault", offer_key.as_ref(), &[self.offer.vault_bump]]];

        transfer(
            CpiContext::new_with_signer(
                self.system_program.to_account_info(),
                Transfer {
                    from: self.offer_vault.to_account_info(),
                    to: self.maker.to_account_info(),
                },
                signers_seeds,
            ),
            maker_amount,
        )?;
        transfer(
            CpiContext::new_with_signer(
                self.system_program.to_account_info(),
                Transfer {
                    from: self.offer_vault.to_account_info(),
                    to: self.treasury.to_account_info(),
                },
                signers_seeds,
            ),
            fee,
        )
    }

    pub fn receive_nft(&mut self) -> Result<()> {
        let asset_key = self.asset.key();

        let bump = self.listing.bump;

        let seed = &[b"listing", asset_key.as_ref(), &[bump]];
        let signers_seeds: &[&[&[u8]]] = &[seed];

        TransferV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(self.collection.as_ref().map(|c| c.as_ref()))
            .payer(&&self.offer_maker.to_account_info())
            .authority(Some(&self.listing.to_account_info()))
            .new_owner(&&self.offer_maker.to_account_info())
            .system_program(Some(&self.system_program.to_account_info()))
            .invoke_signed(signers_seeds)?;

        Ok(())
    }

    pub fn receive_rewards(&mut self) -> Result<()> {
        let seed = &[
            b"maketplace",
            self.maketplace.name.as_bytes(),
            &[self.maketplace.bump],
        ];

        let signer_seeds: &[&[&[u8]]] = &[seed];

        mint_to_checked(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                MintToChecked {
                    to: self.take_reward_ata.to_account_info(),
                    mint: self.reward_mint.to_account_info(),
                    authority: self.maketplace.to_account_info(),
                },
                signer_seeds,
            ),
            100,
            self.reward_mint.decimals,
        )?;
        Ok(())
    }
}
