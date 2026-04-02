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

/// Metaplex Token Metadata program ID
fn metadata_program_id() -> Pubkey {
    "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
        .parse()
        .unwrap()
}

/// SPL Token program ID
fn token_program_id() -> Pubkey {
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        .parse()
        .unwrap()
}

/// Rent sysvar ID
fn rent_sysvar_id() -> Pubkey {
    "SysvarRent111111111111111111111111111111111"
        .parse()
        .unwrap()
}

fn setup() -> (LiteSVM, Pubkey, Keypair) {
    let program_id = create_token::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();

    // Load the program under test
    let program_bytes = include_bytes!("../../../target/deploy/create_token.so");
    svm.add_program(program_id, program_bytes).unwrap();

    // Load the Metaplex Token Metadata program (dumped from mainnet)
    let metadata_bytes = include_bytes!("../../../tests/fixtures/mpl_token_metadata.so");
    svm.add_program(metadata_program_id(), metadata_bytes)
        .unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, program_id, payer)
}

fn send_tx(
    svm: &mut LiteSVM,
    instruction: Instruction,
    payer: &Keypair,
    extra_signers: &[&Keypair],
) {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);
    let mut signers: Vec<&Keypair> = vec![payer];
    signers.extend_from_slice(extra_signers);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &signers).unwrap();
    svm.send_transaction(tx).unwrap();
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

#[test]
fn test_create_spl_token() {
    let (mut svm, program_id, payer) = setup();
    let mint_keypair = Keypair::new();
    let metadata_account = derive_metadata_pda(&mint_keypair.pubkey());

    let instruction = Instruction::new_with_bytes(
        program_id,
        &create_token::instruction::CreateTokenMint {
            _token_decimals: 9,
            token_name: "Solana Gold".to_string(),
            token_symbol: "GOLDSOL".to_string(),
            token_uri: "https://example.com/token.json".to_string(),
        }
        .data(),
        create_token::accounts::CreateTokenMint {
            payer: payer.pubkey(),
            metadata_account,
            mint_account: mint_keypair.pubkey(),
            token_metadata_program: metadata_program_id(),
            token_program: token_program_id(),
            system_program: system_program::id(),
            rent: rent_sysvar_id(),
        }
        .to_account_metas(None),
    );

    send_tx(&mut svm, instruction, &payer, &[&mint_keypair]);

    // Verify the mint account exists
    let mint_account = svm
        .get_account(&mint_keypair.pubkey())
        .expect("Mint account should exist");
    assert!(!mint_account.data.is_empty(), "Mint account should have data");

    // Verify the metadata account was created
    let meta_account = svm
        .get_account(&metadata_account)
        .expect("Metadata account should exist");
    assert!(
        !meta_account.data.is_empty(),
        "Metadata account should have data"
    );
}

#[test]
fn test_create_nft() {
    let (mut svm, program_id, payer) = setup();
    let mint_keypair = Keypair::new();
    let metadata_account = derive_metadata_pda(&mint_keypair.pubkey());

    // NFT uses 0 decimals
    let instruction = Instruction::new_with_bytes(
        program_id,
        &create_token::instruction::CreateTokenMint {
            _token_decimals: 0,
            token_name: "Solana Gold".to_string(),
            token_symbol: "GOLDSOL".to_string(),
            token_uri: "https://example.com/nft.json".to_string(),
        }
        .data(),
        create_token::accounts::CreateTokenMint {
            payer: payer.pubkey(),
            metadata_account,
            mint_account: mint_keypair.pubkey(),
            token_metadata_program: metadata_program_id(),
            token_program: token_program_id(),
            system_program: system_program::id(),
            rent: rent_sysvar_id(),
        }
        .to_account_metas(None),
    );

    send_tx(&mut svm, instruction, &payer, &[&mint_keypair]);

    // Verify the mint account exists
    let mint_account = svm
        .get_account(&mint_keypair.pubkey())
        .expect("Mint account should exist");
    assert!(!mint_account.data.is_empty(), "Mint account should have data");
}
