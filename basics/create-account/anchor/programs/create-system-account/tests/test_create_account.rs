use {
    anchor_lang::{
        solana_program::{instruction::Instruction, rent::Rent, system_program, sysvar::SysvarId},
        InstructionData, ToAccountMetas,
    },
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

#[test]
fn test_create_the_account() {
    let program_id = create_system_account::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/create_system_account.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let new_account = Keypair::new();

    let instruction = Instruction::new_with_bytes(
        program_id,
        &create_system_account::instruction::CreateSystemAccount {}.data(),
        create_system_account::accounts::CreateSystemAccount {
            payer: payer.pubkey(),
            new_account: new_account.pubkey(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &new_account],
    )
    .unwrap();
    svm.send_transaction(tx).unwrap();

    // Minimum balance for rent exemption for 0-data account
    let lamports = Rent::default().minimum_balance(0);

    let account_info = svm.get_account(&new_account.pubkey()).unwrap();
    assert_eq!(account_info.lamports, lamports);
}
