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
    let program_id = mint_close_authority::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/mint_close_authority.so");
    svm.add_program(program_id, program_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, program_id, payer)
}

#[test]
fn test_create_and_close_mint() {
    let (mut svm, program_id, payer) = setup();
    let mint_keypair = Keypair::new();

    // Step 1: Create Mint with Close Authority
    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &mint_close_authority::instruction::Initialize {}.data(),
        mint_close_authority::accounts::Initialize {
            payer: payer.pubkey(),
            mint_account: mint_keypair.pubkey(),
            token_program: token_2022_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, &[initialize_ix], &payer, &[&mint_keypair]);

    // Verify mint exists
    let mint_account = svm
        .get_account(&mint_keypair.pubkey())
        .expect("Mint account should exist");
    assert!(!mint_account.data.is_empty(), "Mint should have data");

    svm.expire_blockhash();

    // Step 2: Close Mint using Anchor CPI
    let close_ix = Instruction::new_with_bytes(
        program_id,
        &mint_close_authority::instruction::Close {}.data(),
        mint_close_authority::accounts::Close {
            authority: payer.pubkey(),
            mint_account: mint_keypair.pubkey(),
            token_program: token_2022_program_id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, &[close_ix], &payer, &[]);

    // Verify mint no longer exists (lamports returned to authority)
    let mint_account = svm.get_account(&mint_keypair.pubkey());
    assert!(
        mint_account.is_none(),
        "Mint account should be closed"
    );

    svm.expire_blockhash();

    // Step 3: Create Mint with Close Authority again (re-use same keypair)
    let initialize_ix2 = Instruction::new_with_bytes(
        program_id,
        &mint_close_authority::instruction::Initialize {}.data(),
        mint_close_authority::accounts::Initialize {
            payer: payer.pubkey(),
            mint_account: mint_keypair.pubkey(),
            token_program: token_2022_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, &[initialize_ix2], &payer, &[&mint_keypair]);

    // Verify mint exists again
    let mint_account = svm
        .get_account(&mint_keypair.pubkey())
        .expect("Mint account should exist after re-creation");
    assert!(
        !mint_account.data.is_empty(),
        "Mint should have data after re-creation"
    );

    svm.expire_blockhash();

    // Step 4: Close Mint directly using Token-2022 CloseAccount instruction
    // Token-2022 CloseAccount is instruction 9
    let close_direct_ix = Instruction {
        program_id: token_2022_program_id(),
        accounts: vec![
            anchor_lang::solana_program::instruction::AccountMeta::new(
                mint_keypair.pubkey(),
                false,
            ),
            anchor_lang::solana_program::instruction::AccountMeta::new(
                payer.pubkey(),
                false,
            ),
            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                payer.pubkey(),
                true,
            ),
        ],
        data: vec![9], // CloseAccount
    };
    send_tx(&mut svm, &[close_direct_ix], &payer, &[]);

    // Verify mint is closed again
    let mint_account = svm.get_account(&mint_keypair.pubkey());
    assert!(
        mint_account.is_none(),
        "Mint account should be closed via direct Token-2022 instruction"
    );
}
