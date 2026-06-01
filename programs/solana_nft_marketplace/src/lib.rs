pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("An54AJiZa84tpn2obdm5p5s91fibELNvuePVC92dJdX2");

#[program]
pub mod nft_marketplace {
    use super::*;

    pub fn initliaze(ctx: Context<InitializeAccount>, name: String, fee: u16) -> Result<()> {
        ctx.accounts.initialize(name, fee, ctx.bumps)
    }

    pub fn list(ctx: Context<ListAcoounts>, price: u64) -> Result<()> {
        ctx.accounts.create_list(price, ctx.bumps)
    }

    pub fn buy(ctx: Context<BuyAccounts>) -> Result<()> {
        ctx.accounts.send_sol()?;
        ctx.accounts.receive_nft()?;
        ctx.accounts.receive_rewards()
    }

    pub fn delist(ctx: Context<DelistAccounts>) -> Result<()> {
        ctx.accounts.refund()
    }

    pub fn maker_offer(ctx: Context<MakeOfferAccounts>, price: u64) -> Result<()> {
        ctx.accounts.make_offer(price, ctx.bumps)
    }
    pub fn accept_offer(ctx: Context<AcceptOfferAccounts>) -> Result<()> {
        ctx.accounts.send_sol()?;
        ctx.accounts.receive_nft()?;
        ctx.accounts.receive_rewards()
    }

    pub fn withdraw_fee(ctx: Context<WithdrawFeeAccounts>, amount: u64) -> Result<()> {
        ctx.accounts.withdraw_fee(amount)
    }
}
