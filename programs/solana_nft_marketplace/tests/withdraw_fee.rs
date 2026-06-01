mod common;

use common::*;
use litesvm::LiteSVM;
use solana_instruction::AccountMeta;
use solana_keypair::Keypair;
use solana_nft_marketplace::instruction;
use solana_signer::Signer;

// Create a marketplace owned by `admin`, returning its PDA.
fn init_market(svm: &mut LiteSVM, admin: &Keypair, name: &str) -> solana_pubkey::Pubkey {
    let marketplace = pda(&[b"maketplace", name.as_bytes()]);
    let accts = vec![
        AccountMeta::new(admin.pubkey(), true),
        AccountMeta::new(marketplace, false),
        AccountMeta::new_readonly(pda(&[b"treasury", marketplace.as_ref()]), false),
        AccountMeta::new(pda(&[b"reward_mint", marketplace.as_ref()]), false),
        AccountMeta::new_readonly(TOKEN_PROGRAM, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
    ];
    let data = instruction::Initliaze { name: name.into(), fee: 250 };
    send(svm, ix(data, accts), admin, &[admin]).unwrap();
    marketplace
}

fn withdraw_accounts(admin: &Keypair, marketplace: solana_pubkey::Pubkey) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new(admin.pubkey(), true),
        AccountMeta::new(marketplace, false),
        AccountMeta::new(pda(&[b"treasury", marketplace.as_ref()]), false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
    ]
}

#[test]
fn withdraw_more_than_treasury_fails() {
    let (mut svm, admin) = setup();
    let marketplace = init_market(&mut svm, &admin, "wf-market");

    // Treasury is empty, so any withdrawal exceeds it -> AmountTooMuch.
    let data = instruction::WithdrawFee { amount: 1 };
    let res = send(&mut svm, ix(data, withdraw_accounts(&admin, marketplace)), &admin, &[&admin]);
    assert!(res.is_err());
}

#[test]
fn withdraw_by_non_admin_fails() {
    let (mut svm, admin) = setup();
    let marketplace = init_market(&mut svm, &admin, "auth-market");

    // A different signer fails the has_one = admin constraint.
    let attacker = Keypair::new();
    svm.airdrop(&attacker.pubkey(), 1_000_000_000).unwrap();
    let data = instruction::WithdrawFee { amount: 0 };
    let res = send(
        &mut svm,
        ix(data, withdraw_accounts(&attacker, marketplace)),
        &attacker,
        &[&attacker],
    );
    assert!(res.is_err());
}
