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
fn instructions_sysvar_id() -> Pubkey {
    "Sysvar1nstructions1111111111111111111111111"
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
    let program_id = mint_nft::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/mint_nft.so");
    svm.add_program(program_id, program_bytes).unwrap();

    let metadata_bytes = include_bytes!("../../../tests/fixtures/mpl_token_metadata.so");
    svm.add_program(metadata_program_id(), metadata_bytes)
        .unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, program_id, payer)
}

#[test]
fn test_create_collection() {
    let (mut svm, program_id, payer) = setup();
    let collection_keypair = Keypair::new();

    let (mint_authority, _) =
        Pubkey::find_program_address(&[b"authority"], &program_id);

    let metadata = derive_metadata_pda(&collection_keypair.pubkey());
    let master_edition = derive_edition_pda(&collection_keypair.pubkey());
    let destination = derive_ata(&payer.pubkey(), &collection_keypair.pubkey());

    let instruction = Instruction::new_with_bytes(
        program_id,
        &mint_nft::instruction::CreateCollection {}.data(),
        mint_nft::accounts::CreateCollection {
            user: payer.pubkey(),
            mint: collection_keypair.pubkey(),
            mint_authority,
            metadata,
            master_edition,
            destination,
            system_program: system_program::id(),
            token_program: token_program_id(),
            associated_token_program: ata_program_id(),
            token_metadata_program: metadata_program_id(),
        }
        .to_account_metas(None),
    );

    send_tx(&mut svm, &[instruction], &payer, &[&collection_keypair]);

    // Verify collection mint exists
    let mint_account = svm
        .get_account(&collection_keypair.pubkey())
        .expect("Collection mint should exist");
    assert!(!mint_account.data.is_empty());

    // Verify metadata exists
    let meta_account = svm
        .get_account(&metadata)
        .expect("Metadata should exist");
    assert!(!meta_account.data.is_empty());

    // Verify master edition exists
    let edition_account = svm
        .get_account(&master_edition)
        .expect("Master edition should exist");
    assert!(!edition_account.data.is_empty());

    // Verify 1 token was minted to destination
    let dest_data = svm
        .get_account(&destination)
        .expect("Destination ATA should exist");
    assert_eq!(read_token_amount(&dest_data.data), 1);
}

#[test]
fn test_mint_nft_to_collection() {
    let (mut svm, program_id, payer) = setup();

    let (mint_authority, _) =
        Pubkey::find_program_address(&[b"authority"], &program_id);

    // Step 1: Create the collection
    let collection_keypair = Keypair::new();
    let collection_metadata = derive_metadata_pda(&collection_keypair.pubkey());
    let collection_master_edition = derive_edition_pda(&collection_keypair.pubkey());
    let collection_destination = derive_ata(&payer.pubkey(), &collection_keypair.pubkey());

    let create_collection_ix = Instruction::new_with_bytes(
        program_id,
        &mint_nft::instruction::CreateCollection {}.data(),
        mint_nft::accounts::CreateCollection {
            user: payer.pubkey(),
            mint: collection_keypair.pubkey(),
            mint_authority,
            metadata: collection_metadata,
            master_edition: collection_master_edition,
            destination: collection_destination,
            system_program: system_program::id(),
            token_program: token_program_id(),
            associated_token_program: ata_program_id(),
            token_metadata_program: metadata_program_id(),
        }
        .to_account_metas(None),
    );

    send_tx(
        &mut svm,
        &[create_collection_ix],
        &payer,
        &[&collection_keypair],
    );

    // Step 2: Mint an NFT into the collection
    svm.expire_blockhash();
    let nft_keypair = Keypair::new();
    let nft_metadata = derive_metadata_pda(&nft_keypair.pubkey());
    let nft_master_edition = derive_edition_pda(&nft_keypair.pubkey());
    let nft_destination = derive_ata(&payer.pubkey(), &nft_keypair.pubkey());

    let mint_nft_ix = Instruction::new_with_bytes(
        program_id,
        &mint_nft::instruction::MintNft {}.data(),
        mint_nft::accounts::MintNFT {
            owner: payer.pubkey(),
            mint: nft_keypair.pubkey(),
            destination: nft_destination,
            metadata: nft_metadata,
            master_edition: nft_master_edition,
            mint_authority,
            collection_mint: collection_keypair.pubkey(),
            system_program: system_program::id(),
            token_program: token_program_id(),
            associated_token_program: ata_program_id(),
            token_metadata_program: metadata_program_id(),
        }
        .to_account_metas(None),
    );

    send_tx(&mut svm, &[mint_nft_ix], &payer, &[&nft_keypair]);

    // Verify NFT was minted
    let nft_dest_data = svm
        .get_account(&nft_destination)
        .expect("NFT ATA should exist");
    assert_eq!(read_token_amount(&nft_dest_data.data), 1);

    // Verify NFT metadata exists
    let nft_meta = svm
        .get_account(&nft_metadata)
        .expect("NFT metadata should exist");
    assert!(!nft_meta.data.is_empty());
}

#[test]
fn test_verify_collection() {
    let (mut svm, program_id, payer) = setup();

    let (mint_authority, _) =
        Pubkey::find_program_address(&[b"authority"], &program_id);

    // Step 1: Create collection
    let collection_keypair = Keypair::new();
    let collection_metadata = derive_metadata_pda(&collection_keypair.pubkey());
    let collection_master_edition = derive_edition_pda(&collection_keypair.pubkey());
    let collection_destination = derive_ata(&payer.pubkey(), &collection_keypair.pubkey());

    let create_collection_ix = Instruction::new_with_bytes(
        program_id,
        &mint_nft::instruction::CreateCollection {}.data(),
        mint_nft::accounts::CreateCollection {
            user: payer.pubkey(),
            mint: collection_keypair.pubkey(),
            mint_authority,
            metadata: collection_metadata,
            master_edition: collection_master_edition,
            destination: collection_destination,
            system_program: system_program::id(),
            token_program: token_program_id(),
            associated_token_program: ata_program_id(),
            token_metadata_program: metadata_program_id(),
        }
        .to_account_metas(None),
    );

    send_tx(
        &mut svm,
        &[create_collection_ix],
        &payer,
        &[&collection_keypair],
    );

    // Step 2: Mint NFT
    svm.expire_blockhash();
    let nft_keypair = Keypair::new();
    let nft_metadata = derive_metadata_pda(&nft_keypair.pubkey());
    let nft_master_edition = derive_edition_pda(&nft_keypair.pubkey());
    let nft_destination = derive_ata(&payer.pubkey(), &nft_keypair.pubkey());

    let mint_nft_ix = Instruction::new_with_bytes(
        program_id,
        &mint_nft::instruction::MintNft {}.data(),
        mint_nft::accounts::MintNFT {
            owner: payer.pubkey(),
            mint: nft_keypair.pubkey(),
            destination: nft_destination,
            metadata: nft_metadata,
            master_edition: nft_master_edition,
            mint_authority,
            collection_mint: collection_keypair.pubkey(),
            system_program: system_program::id(),
            token_program: token_program_id(),
            associated_token_program: ata_program_id(),
            token_metadata_program: metadata_program_id(),
        }
        .to_account_metas(None),
    );

    send_tx(&mut svm, &[mint_nft_ix], &payer, &[&nft_keypair]);

    // Step 3: Verify collection
    svm.expire_blockhash();
    let verify_ix = Instruction::new_with_bytes(
        program_id,
        &mint_nft::instruction::VerifyCollection {}.data(),
        mint_nft::accounts::VerifyCollectionMint {
            authority: payer.pubkey(),
            metadata: nft_metadata,
            mint: nft_keypair.pubkey(),
            mint_authority,
            collection_mint: collection_keypair.pubkey(),
            collection_metadata,
            collection_master_edition,
            system_program: system_program::id(),
            sysvar_instruction: instructions_sysvar_id(),
            token_metadata_program: metadata_program_id(),
        }
        .to_account_metas(None),
    );

    send_tx(&mut svm, &[verify_ix], &payer, &[]);

    // Verify the metadata now shows collection as verified
    // The collection info starts at a known offset in metadata V1 data.
    // We just check that the metadata account is still valid and accessible.
    let nft_meta = svm
        .get_account(&nft_metadata)
        .expect("NFT metadata should still exist after verification");
    assert!(!nft_meta.data.is_empty());
}
