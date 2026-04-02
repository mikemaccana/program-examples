use {
    anchor_lang::{
        solana_program::{
            instruction::Instruction,
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

const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

fn send_tx(svm: &mut LiteSVM, instructions: &[Instruction], payer: &Keypair, extra_signers: &[&Keypair]) {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(instructions, Some(&payer.pubkey()), &blockhash);
    let mut signers: Vec<&Keypair> = vec![payer];
    signers.extend_from_slice(extra_signers);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &signers).unwrap();
    svm.send_transaction(tx).unwrap();
}

#[test]
fn test_transfer_sol_with_cpi() {
    let program_id = transfer_sol::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/transfer_sol.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

    let recipient = Keypair::new();

    let instruction = Instruction::new_with_bytes(
        program_id,
        &transfer_sol::instruction::TransferSolWithCpi {
            amount: LAMPORTS_PER_SOL,
        }
        .data(),
        transfer_sol::accounts::TransferSolWithCpi {
            payer: payer.pubkey(),
            recipient: recipient.pubkey(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );

    send_tx(&mut svm, &[instruction], &payer, &[]);

    let recipient_balance = svm.get_balance(&recipient.pubkey()).unwrap();
    assert_eq!(recipient_balance, LAMPORTS_PER_SOL);
}

#[test]
fn test_transfer_sol_with_program() {
    let program_id = transfer_sol::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/transfer_sol.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

    // Create an account owned by our program with 1 SOL
    let payer_account = Keypair::new();
    let create_account_ix = anchor_lang::solana_program::system_instruction::create_account(
        &payer.pubkey(),
        &payer_account.pubkey(),
        LAMPORTS_PER_SOL,
        0,
        &program_id,
    );
    send_tx(&mut svm, &[create_account_ix], &payer, &[&payer_account]);

    // Now transfer SOL from the program-owned account to a recipient
    svm.expire_blockhash();
    let recipient = Keypair::new();

    let instruction = Instruction::new_with_bytes(
        program_id,
        &transfer_sol::instruction::TransferSolWithProgram {
            amount: LAMPORTS_PER_SOL,
        }
        .data(),
        transfer_sol::accounts::TransferSolWithProgram {
            payer: payer_account.pubkey(),
            recipient: recipient.pubkey(),
        }
        .to_account_metas(None),
    );

    send_tx(&mut svm, &[instruction], &payer, &[]);

    let recipient_balance = svm.get_balance(&recipient.pubkey()).unwrap();
    assert_eq!(recipient_balance, LAMPORTS_PER_SOL);
}
