use {
    anchor_lang::{
        solana_program::{instruction::Instruction, pubkey::Pubkey, system_program},
        InstructionData, ToAccountMetas,
    },
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

fn token_program_id() -> Pubkey {
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        .parse()
        .unwrap()
}
fn ata_program_id() -> Pubkey {
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        .parse()
        .unwrap()
}
fn metadata_program_id() -> Pubkey {
    "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
        .parse()
        .unwrap()
}
fn rent_sysvar_id() -> Pubkey {
    "SysvarRent111111111111111111111111111111111"
        .parse()
        .unwrap()
}

/// Derive ATA address manually.
fn derive_ata(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    let (ata, _bump) = Pubkey::find_program_address(
        &[wallet.as_ref(), token_program_id().as_ref(), mint.as_ref()],
        &ata_program_id(),
    );
    ata
}

/// Derive the metadata account PDA for a given mint.
fn derive_metadata_pda(mint: &Pubkey) -> Pubkey {
    let metadata_pid = metadata_program_id();
    let (pda, _bump) = Pubkey::find_program_address(
        &[b"metadata", metadata_pid.as_ref(), mint.as_ref()],
        &metadata_pid,
    );
    pda
}

/// Derive the master edition account PDA for a given mint.
fn derive_edition_pda(mint: &Pubkey) -> Pubkey {
    let metadata_pid = metadata_program_id();
    let (pda, _bump) = Pubkey::find_program_address(
        &[
            b"metadata",
            metadata_pid.as_ref(),
            mint.as_ref(),
            b"edition",
        ],
        &metadata_pid,
    );
    pda
}

/// Read the token amount from raw SPL Token Account data (offset 64..72).
fn read_token_amount(data: &[u8]) -> u64 {
    u64::from_le_bytes(data[64..72].try_into().unwrap())
}

fn send_tx(
    svm: &mut LiteSVM,
    instructions: &[Instruction],
    payer: &Keypair,
    extra_signers: &[&Keypair],
) {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(instructions, Some(&payer.pubkey()), &blockhash);
    let mut signers: Vec<&Keypair> = vec![payer];
    signers.extend_from_slice(extra_signers);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &signers).unwrap();
    svm.send_transaction(tx).unwrap();
}

fn setup() -> (LiteSVM, Pubkey, Keypair) {
    let program_id = nft_minter::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();

    // Load the program under test
    let program_bytes = include_bytes!("../../../target/deploy/nft_minter.so");
    svm.add_program(program_id, program_bytes).unwrap();

    // Load the Metaplex Token Metadata program
    let metadata_bytes = include_bytes!("../../../tests/fixtures/mpl_token_metadata.so");
    svm.add_program(metadata_program_id(), metadata_bytes)
        .unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, program_id, payer)
}

#[test]
fn test_mint_nft() {
    let (mut svm, program_id, payer) = setup();
    let mint_keypair = Keypair::new();

    let metadata_account = derive_metadata_pda(&mint_keypair.pubkey());
    let edition_account = derive_edition_pda(&mint_keypair.pubkey());
    let associated_token_account = derive_ata(&payer.pubkey(), &mint_keypair.pubkey());

    let instruction = Instruction::new_with_bytes(
        program_id,
        &nft_minter::instruction::MintNft {
            nft_name: "Homer NFT".to_string(),
            nft_symbol: "HOMR".to_string(),
            nft_uri: "https://example.com/nft.json".to_string(),
        }
        .data(),
        nft_minter::accounts::CreateToken {
            payer: payer.pubkey(),
            metadata_account,
            edition_account,
            mint_account: mint_keypair.pubkey(),
            associated_token_account,
            token_program: token_program_id(),
            token_metadata_program: metadata_program_id(),
            associated_token_program: ata_program_id(),
            system_program: system_program::id(),
            rent: rent_sysvar_id(),
        }
        .to_account_metas(None),
    );

    send_tx(&mut svm, &[instruction], &payer, &[&mint_keypair]);

    // Verify the mint account exists (NFT = 0 decimals)
    let mint_account = svm
        .get_account(&mint_keypair.pubkey())
        .expect("Mint account should exist");
    assert!(!mint_account.data.is_empty());

    // Verify the metadata account was created
    let meta_account = svm
        .get_account(&metadata_account)
        .expect("Metadata account should exist");
    assert!(!meta_account.data.is_empty());

    // Verify the edition account was created
    let edition = svm
        .get_account(&edition_account)
        .expect("Edition account should exist");
    assert!(!edition.data.is_empty());

    // Verify 1 NFT was minted to the associated token account
    let ata_data = svm
        .get_account(&associated_token_account)
        .expect("ATA should exist");
    let balance = read_token_amount(&ata_data.data);
    assert_eq!(balance, 1, "Should have exactly 1 NFT");
}
