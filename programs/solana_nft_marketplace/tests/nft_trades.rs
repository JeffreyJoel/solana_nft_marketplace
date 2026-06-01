mod common;

use anchor_lang::AccountDeserialize;
use common::*;
use solana_instruction::AccountMeta;
use solana_nft_marketplace::{
    instruction,
    state::{Listing, Offer},
};
use solana_signer::Signer;

const PRICE: u64 = 1_000_000_000; // 1 SOL

#[test]
fn list_escrows_asset_into_listing() {
    let (mut svm, maker) = setup_with_core();
    let asset = create_asset(&mut svm, &maker);

    let listing_pk = list_asset(&mut svm, &maker, &asset, PRICE);

    // Listing PDA records maker/asset/price.
    let acct = svm.get_account(&listing_pk).unwrap();
    let listing = Listing::try_deserialize(&mut acct.data.as_slice()).unwrap();
    assert_eq!(listing.maker.to_bytes(), maker.pubkey().to_bytes());
    assert_eq!(listing.asset.to_bytes(), asset.pubkey().to_bytes());
    assert_eq!(listing.price, PRICE);
}

#[test]
fn delist_closes_listing() {
    let (mut svm, admin) = setup_with_core();
    init_market(&mut svm, &admin, "delist-mkt");
    let maker = funded(&mut svm, 5_000_000_000);
    let asset = create_asset(&mut svm, &maker);
    let listing_pk = list_asset(&mut svm, &maker, &asset, PRICE);

    let accts = vec![
        AccountMeta::new(maker.pubkey(), true),
        AccountMeta::new(asset.pubkey(), false),
        none_account(),
        AccountMeta::new_readonly(pda(&[b"maketplace", b"delist-mkt"]), false),
        AccountMeta::new(listing_pk, false),
        AccountMeta::new_readonly(MPL_CORE, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
    ];
    send(&mut svm, ix(instruction::Delist {}, accts), &maker, &[&maker]).unwrap();

    // close = maker -> listing account is gone.
    assert!(svm.get_account(&listing_pk).is_none());
}

#[test]
fn buy_pays_seller_fee_and_mints_reward() {
    let (mut svm, admin) = setup_with_core();
    let marketplace = init_market(&mut svm, &admin, "buy-mkt");
    let maker = funded(&mut svm, 5_000_000_000);
    let asset = create_asset(&mut svm, &maker);
    let listing_pk = list_asset(&mut svm, &maker, &asset, PRICE);
    let taker = funded(&mut svm, 5_000_000_000);

    let treasury = pda(&[b"treasury", marketplace.as_ref()]);
    let reward_mint = pda(&[b"reward_mint", marketplace.as_ref()]);
    let accts = vec![
        AccountMeta::new(taker.pubkey(), true),
        AccountMeta::new(maker.pubkey(), false),
        AccountMeta::new(asset.pubkey(), false),
        none_account(),
        AccountMeta::new_readonly(marketplace, false),
        AccountMeta::new(listing_pk, false),
        AccountMeta::new(treasury, false),
        AccountMeta::new(reward_mint, false), // writable: mint_to bumps supply
        AccountMeta::new(ata(&taker.pubkey(), &reward_mint), false),
        AccountMeta::new_readonly(MPL_CORE, false),
        AccountMeta::new_readonly(ATA_PROGRAM, false),
        AccountMeta::new_readonly(TOKEN_PROGRAM, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
    ];
    send(&mut svm, ix(instruction::Buy {}, accts), &taker, &[&taker]).unwrap();

    // Listing consumed; treasury holds the 2.5% fee.
    assert!(svm.get_account(&listing_pk).is_none());
    assert_eq!(svm.get_account(&treasury).unwrap().lamports, PRICE / 10_000 * 250);
}

#[test]
fn make_offer_escrows_price_into_vault() {
    let (mut svm, admin) = setup_with_core();
    init_market(&mut svm, &admin, "offer-mkt");
    let maker = funded(&mut svm, 5_000_000_000);
    let asset = create_asset(&mut svm, &maker);
    let listing_pk = list_asset(&mut svm, &maker, &asset, PRICE);
    let buyer = funded(&mut svm, 5_000_000_000);

    let offer_pk = pda(&[b"offer", listing_pk.as_ref(), buyer.pubkey().as_ref()]);
    let vault = pda(&[b"offer_vault", offer_pk.as_ref()]);
    let offer_price = 2_000_000_000;
    let accts = vec![
        AccountMeta::new(buyer.pubkey(), true),
        AccountMeta::new(offer_pk, false),
        AccountMeta::new(vault, false),
        AccountMeta::new(listing_pk, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
    ];
    let data = instruction::MakerOffer { price: offer_price };
    send(&mut svm, ix(data, accts), &buyer, &[&buyer]).unwrap();

    // Offer PDA records the bid; vault holds the escrowed lamports.
    let acct = svm.get_account(&offer_pk).unwrap();
    let offer = Offer::try_deserialize(&mut acct.data.as_slice()).unwrap();
    assert_eq!(offer.offer_maker.to_bytes(), buyer.pubkey().to_bytes());
    assert_eq!(offer.price, offer_price);
    assert_eq!(svm.get_account(&vault).unwrap().lamports, offer_price);
}

#[test]
fn accept_offer_settles_trade() {
    let (mut svm, admin) = setup_with_core();
    let marketplace = init_market(&mut svm, &admin, "accept-mkt");
    let maker = funded(&mut svm, 5_000_000_000);
    let asset = create_asset(&mut svm, &maker);
    let listing_pk = list_asset(&mut svm, &maker, &asset, PRICE);
    let buyer = funded(&mut svm, 5_000_000_000);

    // Buyer escrows a 2 SOL offer.
    let offer_price = 2_000_000_000u64;
    let offer_pk = pda(&[b"offer", listing_pk.as_ref(), buyer.pubkey().as_ref()]);
    let vault = pda(&[b"offer_vault", offer_pk.as_ref()]);
    let offer_accts = vec![
        AccountMeta::new(buyer.pubkey(), true),
        AccountMeta::new(offer_pk, false),
        AccountMeta::new(vault, false),
        AccountMeta::new(listing_pk, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
    ];
    send(&mut svm, ix(instruction::MakerOffer { price: offer_price }, offer_accts), &buyer, &[&buyer]).unwrap();

    // Seller accepts; buyer co-signs to fund its reward ATA.
    let treasury = pda(&[b"treasury", marketplace.as_ref()]);
    let reward_mint = pda(&[b"reward_mint", marketplace.as_ref()]);
    let accts = vec![
        AccountMeta::new(buyer.pubkey(), true),  // offer_maker: pays ATA rent, gets offer rent back
        AccountMeta::new(maker.pubkey(), true),  // maker: seller
        AccountMeta::new(offer_pk, false),
        AccountMeta::new(vault, false),
        AccountMeta::new(asset.pubkey(), false),
        none_account(),
        AccountMeta::new_readonly(marketplace, false),
        AccountMeta::new(listing_pk, false),
        AccountMeta::new(treasury, false),
        AccountMeta::new(reward_mint, false),
        AccountMeta::new(ata(&buyer.pubkey(), &reward_mint), false),
        AccountMeta::new_readonly(MPL_CORE, false),
        AccountMeta::new_readonly(ATA_PROGRAM, false),
        AccountMeta::new_readonly(TOKEN_PROGRAM, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
    ];
    send(&mut svm, ix(instruction::AcceptOffer {}, accts), &maker, &[&maker, &buyer]).unwrap();

    // Offer + listing consumed; treasury holds the 2.5% fee of the escrowed bid.
    assert!(svm.get_account(&offer_pk).is_none());
    assert!(svm.get_account(&listing_pk).is_none());
    assert_eq!(svm.get_account(&treasury).unwrap().lamports, offer_price / 10_000 * 250);
    // Buyer received 100 reward tokens (SPL amount at byte offset 64).
    let ata_data = svm.get_account(&ata(&buyer.pubkey(), &reward_mint)).unwrap().data;
    assert_eq!(u64::from_le_bytes(ata_data[64..72].try_into().unwrap()), 100);
}
