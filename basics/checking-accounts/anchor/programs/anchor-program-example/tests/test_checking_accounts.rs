use {
    anchor_lang::{
        solana_program::{
            instruction::Instruction,
            system_instruction,
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

#[test]
fn test_check_accounts() {
    let program_id = checking_account_program::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/checking_account_program.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let account_to_change = Keypair::new();
    let account_to_create = Keypair::new();

    // First, create an account owned by our program (like the TS test does)
    let rent_exempt_balance = svm.minimum_balance_for_rent_exemption(0);
    let create_account_ix = system_instruction::create_account(
        &payer.pubkey(),
        &account_to_change.pubkey(),
        rent_exempt_balance,
        0,
        &program_id,
    );
    send_tx(&mut svm, create_account_ix, &payer, &[&account_to_change]);

    svm.expire_blockhash();

    // Now call check_accounts
    let check_accounts_ix = Instruction::new_with_bytes(
        program_id,
        &checking_account_program::instruction::CheckAccounts {}.data(),
        checking_account_program::accounts::CheckingAccounts {
            payer: payer.pubkey(),
            account_to_create: account_to_create.pubkey(),
            account_to_change: account_to_change.pubkey(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, check_accounts_ix, &payer, &[]);
}
