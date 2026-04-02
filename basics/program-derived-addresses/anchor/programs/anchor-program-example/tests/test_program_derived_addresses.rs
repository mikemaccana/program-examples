use {
    anchor_lang::{
        solana_program::{
            instruction::Instruction,
            pubkey::Pubkey,
            system_program,
        },
        InstructionData, ToAccountMetas,
    },
    borsh::BorshDeserialize,
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
    let program_id = program_derived_addresses_program::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/program_derived_addresses_program.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, payer)
}

#[derive(BorshDeserialize)]
struct PageVisits {
    page_visits: u32,
    bump: u8,
}

#[test]
fn test_create_and_increment_page_visits() {
    let (mut svm, payer) = setup();
    let program_id = program_derived_addresses_program::id();

    // Derive PDA
    let (page_visits_pda, _bump) = Pubkey::find_program_address(
        &[b"page_visits", payer.pubkey().as_ref()],
        &program_id,
    );

    // Create page visits account
    let create_ix = Instruction::new_with_bytes(
        program_id,
        &program_derived_addresses_program::instruction::CreatePageVisits {}.data(),
        program_derived_addresses_program::accounts::CreatePageVisits {
            payer: payer.pubkey(),
            page_visits: page_visits_pda,
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, create_ix, &payer, &[]);

    // Verify initial state (page_visits = 0)
    let account = svm.get_account(&page_visits_pda).expect("PDA should exist");
    let data = PageVisits::try_from_slice(&account.data[8..]).unwrap();
    assert_eq!(data.page_visits, 0, "Initial page visits should be 0");

    svm.expire_blockhash();

    // Increment page visits
    let increment_ix = Instruction::new_with_bytes(
        program_id,
        &program_derived_addresses_program::instruction::IncrementPageVisits {}.data(),
        program_derived_addresses_program::accounts::IncrementPageVisits {
            user: payer.pubkey(),
            page_visits: page_visits_pda,
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, increment_ix, &payer, &[]);

    // Verify page_visits = 1
    let account = svm.get_account(&page_visits_pda).expect("PDA should exist");
    let data = PageVisits::try_from_slice(&account.data[8..]).unwrap();
    assert_eq!(data.page_visits, 1, "Page visits should be 1 after increment");

    svm.expire_blockhash();

    // Increment again
    let increment_ix2 = Instruction::new_with_bytes(
        program_id,
        &program_derived_addresses_program::instruction::IncrementPageVisits {}.data(),
        program_derived_addresses_program::accounts::IncrementPageVisits {
            user: payer.pubkey(),
            page_visits: page_visits_pda,
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, increment_ix2, &payer, &[]);

    // Verify page_visits = 2
    let account = svm.get_account(&page_visits_pda).expect("PDA should exist");
    let data = PageVisits::try_from_slice(&account.data[8..]).unwrap();
    assert_eq!(data.page_visits, 2, "Page visits should be 2 after second increment");
}
