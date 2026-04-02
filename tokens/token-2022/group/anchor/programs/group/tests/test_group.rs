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

fn token_2022_program_id() -> Pubkey {
    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
        .parse()
        .unwrap()
}

#[test]
fn test_initialize_group() {
    let program_id = group::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/group.so");
    svm.add_program(program_id, program_bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    // Derive the mint PDA
    let (mint_account, _bump) = Pubkey::find_program_address(&[b"group"], &program_id);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &group::instruction::TestInitializeGroup {}.data(),
        group::accounts::InitializeGroup {
            payer: payer.pubkey(),
            mint_account,
            token_program: token_2022_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer]).unwrap();
    svm.send_transaction(tx).unwrap();

    // Verify mint was created with group pointer extension
    let mint_data = svm
        .get_account(&mint_account)
        .expect("Mint account should exist");
    assert!(
        mint_data.data.len() > 82,
        "Mint should have extension data (size > 82, got {})",
        mint_data.data.len()
    );
}
