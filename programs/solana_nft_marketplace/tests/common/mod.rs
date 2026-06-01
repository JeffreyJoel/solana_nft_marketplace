// Shared LiteSVM harness for the marketplace integration tests.
use anchor_lang::InstructionData;
use litesvm::LiteSVM;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::{Message, VersionedMessage};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;

pub const TOKEN_PROGRAM: Pubkey =
    solana_pubkey::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
pub const SYSTEM_PROGRAM: Pubkey = solana_pubkey::pubkey!("11111111111111111111111111111111");
pub const ATA_PROGRAM: Pubkey =
    solana_pubkey::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
pub const MPL_CORE: Pubkey = solana_pubkey::pubkey!("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d");

/// Anchor's optional-account sentinel: passing the program id means `None`.
pub fn none_account() -> AccountMeta {
    AccountMeta::new_readonly(program_id(), false)
}

/// Associated token account for (owner, mint).
pub fn ata(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[owner.as_ref(), TOKEN_PROGRAM.as_ref(), mint.as_ref()],
        &ATA_PROGRAM,
    )
    .0
}

/// Program id as a v3 pubkey (anchor's `id()` is on the older solana crate).
pub fn program_id() -> Pubkey {
    Pubkey::new_from_array(solana_nft_marketplace::id().to_bytes())
}

/// Fresh SVM with the built program loaded and `payer` funded.
pub fn setup() -> (LiteSVM, Keypair) {
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../../target/deploy/solana_nft_marketplace.so");
    svm.add_program(program_id(), bytes).unwrap();
    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, payer)
}

/// Same as `setup` but also loads the Metaplex Core program (for NFT CPIs).
pub fn setup_with_core() -> (LiteSVM, Keypair) {
    let (mut svm, payer) = setup();
    let core = include_bytes!("../fixtures/mpl_core.so");
    svm.add_program(MPL_CORE, core).unwrap();
    (svm, payer)
}

/// A freshly funded keypair.
pub fn funded(svm: &mut LiteSVM, lamports: u64) -> Keypair {
    let kp = Keypair::new();
    svm.airdrop(&kp.pubkey(), lamports).unwrap();
    kp
}

fn to_core(pk: &Pubkey) -> anchor_lang::prelude::Pubkey {
    anchor_lang::prelude::Pubkey::new_from_array(pk.to_bytes())
}

/// Re-encode an mpl-core (solana 2.x) instruction into the litesvm (3.x) type.
fn from_core(ix: anchor_lang::solana_program::instruction::Instruction) -> Instruction {
    let metas = ix
        .accounts
        .iter()
        .map(|m| {
            let pk = Pubkey::new_from_array(m.pubkey.to_bytes());
            if m.is_writable {
                AccountMeta::new(pk, m.is_signer)
            } else {
                AccountMeta::new_readonly(pk, m.is_signer)
            }
        })
        .collect();
    Instruction::new_with_bytes(Pubkey::new_from_array(ix.program_id.to_bytes()), &ix.data, metas)
}

/// Mint a standalone Core asset owned by `owner`; returns the asset keypair.
pub fn create_asset(svm: &mut LiteSVM, owner: &Keypair) -> Keypair {
    use mpl_core::{instructions::CreateV1Builder, types::DataState};
    let asset = Keypair::new();
    let core_ix = CreateV1Builder::new()
        .asset(to_core(&asset.pubkey()))
        .payer(to_core(&owner.pubkey()))
        .owner(Some(to_core(&owner.pubkey())))
        .data_state(DataState::AccountState)
        .name("Test NFT".into())
        .uri("https://example.com/nft.json".into())
        .instruction();
    send(svm, from_core(core_ix), owner, &[owner, &asset]).unwrap();
    asset
}

pub fn pda(seeds: &[&[u8]]) -> Pubkey {
    Pubkey::find_program_address(seeds, &program_id()).0
}

/// Anchor-encoded ix data (8-byte discriminator + borsh args).
pub fn ix(data: impl InstructionData, accounts: Vec<AccountMeta>) -> Instruction {
    Instruction::new_with_bytes(program_id(), &data.data(), accounts)
}

/// Create a marketplace owned by `admin` (fee = 250 bps); returns its PDA.
pub fn init_market(svm: &mut LiteSVM, admin: &Keypair, name: &str) -> Pubkey {
    use solana_nft_marketplace::instruction::Initliaze;
    let marketplace = pda(&[b"maketplace", name.as_bytes()]);
    let accts = vec![
        AccountMeta::new(admin.pubkey(), true),
        AccountMeta::new(marketplace, false),
        AccountMeta::new_readonly(pda(&[b"treasury", marketplace.as_ref()]), false),
        AccountMeta::new(pda(&[b"reward_mint", marketplace.as_ref()]), false),
        AccountMeta::new_readonly(TOKEN_PROGRAM, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
    ];
    let data = Initliaze { name: name.into(), fee: 250 };
    send(svm, ix(data, accts), admin, &[admin]).unwrap();
    marketplace
}

/// List `asset` (owned by `maker`) at `price`; returns the listing PDA.
pub fn list_asset(svm: &mut LiteSVM, maker: &Keypair, asset: &Keypair, price: u64) -> Pubkey {
    use solana_nft_marketplace::instruction::List;
    let listing = pda(&[b"listing", asset.pubkey().as_ref()]);
    let accts = vec![
        AccountMeta::new(maker.pubkey(), true),
        AccountMeta::new(asset.pubkey(), false),
        none_account(),
        AccountMeta::new(listing, false),
        AccountMeta::new_readonly(MPL_CORE, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
    ];
    send(svm, ix(List { price }, accts), maker, &[maker]).unwrap();
    listing
}

/// Sign and send a single instruction; returns the SVM result.
pub fn send(
    svm: &mut LiteSVM,
    ix: Instruction,
    payer: &Keypair,
    signers: &[&Keypair],
) -> Result<(), litesvm::types::FailedTransactionMetadata> {
    let msg = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &svm.latest_blockhash());
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), signers).unwrap();
    svm.send_transaction(tx).map(|_| ())
}
