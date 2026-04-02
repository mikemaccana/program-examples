use {
    anchor_lang::{
        solana_program::{
            instruction::Instruction,
            pubkey::Pubkey,
            system_program,
        },
        InstructionData, ToAccountMetas,
    },
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

fn send_tx(svm: &mut LiteSVM, instruction: Instruction, payer: &Keypair, signers: &[&Keypair]) {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);
    let mut all_signers: Vec<&Keypair> = vec![payer];
    all_signers.extend_from_slice(signers);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &all_signers).unwrap();
    svm.send_transaction(tx).unwrap();
}

fn setup() -> (LiteSVM, Keypair) {
    let program_id = close_account_program::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/close_account_program.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, payer)
}

#[test]
fn test_create_and_close_user() {
    let (mut svm, payer) = setup();
    let program_id = close_account_program::id();

    // Derive the PDA for the user's account
    let (user_account_pda, _bump) = Pubkey::find_program_address(
        &[b"USER", payer.pubkey().as_ref()],
        &program_id,
    );

    // Create user
    let create_ix = Instruction::new_with_bytes(
        program_id,
        &close_account_program::instruction::CreateUser {
            name: "John Doe".to_string(),
        }
        .data(),
        close_account_program::accounts::CreateUserContext {
            user: payer.pubkey(),
            user_account: user_account_pda,
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, create_ix, &payer, &[]);

    // Verify account exists and has correct data
    let account = svm.get_account(&user_account_pda).expect("Account should exist after creation");
    assert!(account.data.len() > 0, "Account should have data");

    svm.expire_blockhash();

    // Close user
    let close_ix = Instruction::new_with_bytes(
        program_id,
        &close_account_program::instruction::CloseUser {}.data(),
        close_account_program::accounts::CloseUserContext {
            user: payer.pubkey(),
            user_account: user_account_pda,
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, close_ix, &payer, &[]);

    // Verify account is closed
    let account = svm.get_account(&user_account_pda);
    assert!(account.is_none(), "Account should be closed");
}
