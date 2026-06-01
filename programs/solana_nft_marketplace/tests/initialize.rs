mod common;

use anchor_lang::AccountDeserialize;
use common::*;
use solana_instruction::AccountMeta;
use solana_nft_marketplace::{instruction, state::MarketPlace};
use solana_signer::Signer;

// Account metas for `initliaze`, in declaration order.
fn accounts(admin: &solana_keypair::Keypair, name: &str) -> Vec<AccountMeta> {
    let marketplace = pda(&[b"maketplace", name.as_bytes()]);
    vec![
        AccountMeta::new(admin.pubkey(), true),
        AccountMeta::new(marketplace, false),
        AccountMeta::new_readonly(pda(&[b"treasury", marketplace.as_ref()]), false),
        AccountMeta::new(pda(&[b"reward_mint", marketplace.as_ref()]), false),
        AccountMeta::new_readonly(TOKEN_PROGRAM, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
    ]
}

#[test]
fn initialize_ok() {
    let (mut svm, admin) = setup();
    let name = "test-market";
    let data = instruction::Initliaze { name: name.into(), fee: 250 };

    send(&mut svm, ix(data, accounts(&admin, name)), &admin, &[&admin]).unwrap();

    // Marketplace PDA holds the supplied config.
    let acct = svm.get_account(&pda(&[b"maketplace", name.as_bytes()])).unwrap();
    let mp = MarketPlace::try_deserialize(&mut acct.data.as_slice()).unwrap();
    assert_eq!(mp.admin.to_bytes(), admin.pubkey().to_bytes());
    assert_eq!(mp.fee, 250);
    assert_eq!(mp.name, name);
}

#[test]
fn initialize_rejects_fee_over_max() {
    let (mut svm, admin) = setup();
    let name = "bad-fee";
    let data = instruction::Initliaze { name: name.into(), fee: 10_001 };

    // fee > 10_000 trips the InvalidFee guard.
    let res = send(&mut svm, ix(data, accounts(&admin, name)), &admin, &[&admin]);
    assert!(res.is_err());
}
