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
    let program_id = pda_rent_payer::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/pda_rent_payer.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, payer)
}

#[test]
fn test_init_rent_vault() {
    let (mut svm, payer) = setup();
    let program_id = pda_rent_payer::id();

    let (rent_vault_pda, _bump) = Pubkey::find_program_address(
        &[b"rent_vault"],
        &program_id,
    );

    // Fund the rent vault with 1 SOL
    let fund_amount: u64 = 1_000_000_000;
    let init_ix = Instruction::new_with_bytes(
        program_id,
        &pda_rent_payer::instruction::InitRentVault {
            fund_lamports: fund_amount,
        }
        .data(),
        pda_rent_payer::accounts::InitRentVault {
            payer: payer.pubkey(),
            rent_vault: rent_vault_pda,
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, init_ix, &payer, &[]);

    // Verify the rent vault has the correct balance
    let account = svm.get_account(&rent_vault_pda).expect("Rent vault should exist");
    assert_eq!(account.lamports, fund_amount, "Rent vault should have 1 SOL");
}

#[test]
fn test_create_new_account_from_rent_vault() {
    let (mut svm, payer) = setup();
    let program_id = pda_rent_payer::id();

    let (rent_vault_pda, _bump) = Pubkey::find_program_address(
        &[b"rent_vault"],
        &program_id,
    );

    // Fund the rent vault with 1 SOL
    let fund_amount: u64 = 1_000_000_000;
    let init_ix = Instruction::new_with_bytes(
        program_id,
        &pda_rent_payer::instruction::InitRentVault {
            fund_lamports: fund_amount,
        }
        .data(),
        pda_rent_payer::accounts::InitRentVault {
            payer: payer.pubkey(),
            rent_vault: rent_vault_pda,
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, init_ix, &payer, &[]);

    svm.expire_blockhash();

    // Create a new account using the rent vault
    let new_account = Keypair::new();
    let create_ix = Instruction::new_with_bytes(
        program_id,
        &pda_rent_payer::instruction::CreateNewAccount {}.data(),
        pda_rent_payer::accounts::CreateNewAccount {
            new_account: new_account.pubkey(),
            rent_vault: rent_vault_pda,
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, create_ix, &payer, &[&new_account]);

    // Verify the new account was created with minimum rent-exempt balance
    let rent_exempt_balance = svm.minimum_balance_for_rent_exemption(0);
    let account = svm.get_account(&new_account.pubkey()).expect("New account should exist");
    assert_eq!(
        account.lamports, rent_exempt_balance,
        "New account should have rent-exempt balance"
    );
}
