use {
    anchor_lang::{
        solana_program::instruction::Instruction,
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
    let program_id = processing_instructions::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/processing_instructions.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, payer)
}

#[test]
fn test_go_to_park() {
    let (mut svm, payer) = setup();
    let program_id = processing_instructions::id();

    // Test with short person (height 3)
    let ix_short = Instruction::new_with_bytes(
        program_id,
        &processing_instructions::instruction::GoToPark {
            name: "Jimmy".to_string(),
            height: 3,
        }
        .data(),
        processing_instructions::accounts::Park {}
            .to_account_metas(None),
    );
    send_tx(&mut svm, ix_short, &payer, &[]);

    svm.expire_blockhash();

    // Test with tall person (height 10)
    let ix_tall = Instruction::new_with_bytes(
        program_id,
        &processing_instructions::instruction::GoToPark {
            name: "Mary".to_string(),
            height: 10,
        }
        .data(),
        processing_instructions::accounts::Park {}
            .to_account_metas(None),
    );
    send_tx(&mut svm, ix_tall, &payer, &[]);
}
