use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Name it too long")]
    InvalidName,
    #[msg("Fees is too much")]
    InvalidFee,
    #[msg("Amount is greter then treasury fund")]
    AmountTooMuch,
    #[msg("Nft already solded")]
    AlreadySold,
    #[msg("Offer not accepted")]
    OfferNotAccepted,
}
