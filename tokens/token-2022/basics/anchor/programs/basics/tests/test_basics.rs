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

fn ata_program_id() -> Pubkey {
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        .parse()
        .unwrap()
}

/// Derive ATA address for Token-2022.
fn derive_ata(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    let (ata, _bump) = Pubkey::find_program_address(
        &[
            wallet.as_ref(),
            token_2022_program_id().as_ref(),
            mint.as_ref(),
        ],
        &ata_program_id(),
    );
    ata
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
    let program_id = anchor::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/anchor.so");
    svm.add_program(program_id, program_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, program_id, payer)
}

#[test]
fn test_create_token_and_mint_and_transfer() {
    let (mut svm, program_id, payer) = setup();

    let token_name = "TestToken".to_string();

    // Derive the mint PDA
    let (mint, _bump) = Pubkey::find_program_address(
        &[
            b"token-2022-token",
            payer.pubkey().as_ref(),
            token_name.as_bytes(),
        ],
        &program_id,
    );

    // Step 1: Create Token
    let create_token_ix = Instruction::new_with_bytes(
        program_id,
        &anchor::instruction::CreateToken {
            _token_name: token_name.clone(),
        }
        .data(),
        anchor::accounts::CreateToken {
            signer: payer.pubkey(),
            mint,
            system_program: system_program::id(),
            token_program: token_2022_program_id(),
        }
        .to_account_metas(None),
    );

    send_tx(&mut svm, &[create_token_ix], &payer, &[]);

    // Verify mint account exists
    let mint_account = svm.get_account(&mint).expect("Mint account should exist");
    assert!(!mint_account.data.is_empty(), "Mint should have data");

    svm.expire_blockhash();

    // Step 2: Create Associated Token Account for payer
    let payer_ata = derive_ata(&payer.pubkey(), &mint);

    let create_ata_ix = Instruction::new_with_bytes(
        program_id,
        &anchor::instruction::CreateAssociatedTokenAccount {}.data(),
        anchor::accounts::CreateAssociatedTokenAccount {
            signer: payer.pubkey(),
            mint,
            token_account: payer_ata,
            system_program: system_program::id(),
            token_program: token_2022_program_id(),
            associated_token_program: ata_program_id(),
        }
        .to_account_metas(None),
    );

    send_tx(&mut svm, &[create_ata_ix], &payer, &[]);

    // Verify ATA exists
    let ata_account = svm
        .get_account(&payer_ata)
        .expect("Payer ATA should exist");
    assert!(!ata_account.data.is_empty(), "ATA should have data");

    svm.expire_blockhash();

    // Step 3: Mint tokens to payer ATA
    let mint_amount: u64 = 200_000_000;

    let mint_token_ix = Instruction::new_with_bytes(
        program_id,
        &anchor::instruction::MintToken {
            amount: mint_amount,
        }
        .data(),
        anchor::accounts::MintToken {
            signer: payer.pubkey(),
            mint,
            receiver: payer_ata,
            token_program: token_2022_program_id(),
        }
        .to_account_metas(None),
    );

    send_tx(&mut svm, &[mint_token_ix], &payer, &[]);

    // Verify tokens were minted — read amount from token account data
    // Token-2022 account layout: amount is at offset 64..72 (same as SPL Token)
    let ata_data = svm.get_account(&payer_ata).unwrap();
    let amount = u64::from_le_bytes(ata_data.data[64..72].try_into().unwrap());
    assert_eq!(amount, mint_amount, "Should have minted {} tokens", mint_amount);

    svm.expire_blockhash();

    // Step 4: Transfer tokens to receiver
    let receiver = Keypair::new();
    svm.airdrop(&receiver.pubkey(), 1_000_000_000).unwrap();
    let receiver_ata = derive_ata(&receiver.pubkey(), &mint);

    let transfer_amount: u64 = 100;

    let transfer_ix = Instruction::new_with_bytes(
        program_id,
        &anchor::instruction::TransferToken {
            amount: transfer_amount,
        }
        .data(),
        anchor::accounts::TransferToken {
            signer: payer.pubkey(),
            from: payer_ata,
            to: receiver.pubkey(),
            to_ata: receiver_ata,
            mint,
            token_program: token_2022_program_id(),
            system_program: system_program::id(),
            associated_token_program: ata_program_id(),
        }
        .to_account_metas(None),
    );

    send_tx(&mut svm, &[transfer_ix], &payer, &[]);

    // Verify transfer succeeded
    let receiver_ata_data = svm.get_account(&receiver_ata).unwrap();
    let receiver_amount =
        u64::from_le_bytes(receiver_ata_data.data[64..72].try_into().unwrap());
    assert_eq!(
        receiver_amount, transfer_amount,
        "Receiver should have {} tokens",
        transfer_amount
    );

    let payer_ata_data = svm.get_account(&payer_ata).unwrap();
    let payer_remaining = u64::from_le_bytes(payer_ata_data.data[64..72].try_into().unwrap());
    assert_eq!(
        payer_remaining,
        mint_amount - transfer_amount,
        "Payer should have remaining tokens"
    );
}
