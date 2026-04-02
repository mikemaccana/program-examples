use {
    anchor_lang::{
        solana_program::{
            instruction::{AccountMeta, Instruction},
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

fn token_2022_program_id() -> Pubkey {
    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
        .parse()
        .unwrap()
}

fn associated_token_program_id() -> Pubkey {
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
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

fn try_send_tx(
    svm: &mut LiteSVM,
    instructions: &[Instruction],
    payer: &Keypair,
    extra_signers: &[&Keypair],
) -> Result<litesvm::types::TransactionMetadata, litesvm::types::FailedTransactionMetadata> {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(instructions, Some(&payer.pubkey()), &blockhash);
    let mut signers: Vec<&Keypair> = vec![payer];
    signers.extend_from_slice(extra_signers);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &signers).unwrap();
    svm.send_transaction(tx)
}

fn setup() -> (LiteSVM, Pubkey, Keypair) {
    let program_id = non_transferable::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/non_transferable.so");
    svm.add_program(program_id, program_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, program_id, payer)
}

/// Derive the associated token address for Token-2022.
fn get_associated_token_address(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    let token_2022 = token_2022_program_id();
    let ata_program = associated_token_program_id();
    let (address, _bump) = Pubkey::find_program_address(
        &[
            wallet.as_ref(),
            token_2022.as_ref(),
            mint.as_ref(),
        ],
        &ata_program,
    );
    address
}

/// Create an associated token account for Token-2022.
fn create_associated_token_account_ix(
    payer: &Pubkey,
    wallet: &Pubkey,
    mint: &Pubkey,
) -> Instruction {
    let ata = get_associated_token_address(wallet, mint);
    let token_2022 = token_2022_program_id();
    let ata_program = associated_token_program_id();
    Instruction {
        program_id: ata_program,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(ata, false),
            AccountMeta::new_readonly(*wallet, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(token_2022, false),
        ],
        data: vec![],
    }
}

/// MintTo instruction for Token-2022 (instruction 7).
fn mint_to_ix(
    mint: &Pubkey,
    destination: &Pubkey,
    authority: &Pubkey,
    amount: u64,
) -> Instruction {
    let token_2022 = token_2022_program_id();
    let mut data = vec![7u8];
    data.extend_from_slice(&amount.to_le_bytes());
    Instruction {
        program_id: token_2022,
        accounts: vec![
            AccountMeta::new(*mint, false),
            AccountMeta::new(*destination, false),
            AccountMeta::new_readonly(*authority, true),
        ],
        data,
    }
}

/// TransferChecked instruction for Token-2022 (instruction 12).
fn transfer_checked_ix(
    source: &Pubkey,
    mint: &Pubkey,
    destination: &Pubkey,
    authority: &Pubkey,
    amount: u64,
    decimals: u8,
) -> Instruction {
    let token_2022 = token_2022_program_id();
    let mut data = vec![12u8];
    data.extend_from_slice(&amount.to_le_bytes());
    data.push(decimals);
    Instruction {
        program_id: token_2022,
        accounts: vec![
            AccountMeta::new(*source, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new(*destination, false),
            AccountMeta::new_readonly(*authority, true),
        ],
        data,
    }
}

#[test]
fn test_create_non_transferable_mint_and_attempt_transfer() {
    let (mut svm, program_id, payer) = setup();
    let mint_keypair = Keypair::new();
    let token_2022 = token_2022_program_id();

    // Step 1: Create mint with NonTransferable extension via our program
    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &non_transferable::instruction::Initialize {}.data(),
        non_transferable::accounts::Initialize {
            payer: payer.pubkey(),
            mint_account: mint_keypair.pubkey(),
            token_program: token_2022,
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, &[initialize_ix], &payer, &[&mint_keypair]);
    svm.expire_blockhash();

    // Verify mint account was created and has extension data
    let mint_data = svm
        .get_account(&mint_keypair.pubkey())
        .expect("Mint account should exist");
    assert!(
        mint_data.data.len() > 82,
        "Mint should have extension data (size > 82, got {})",
        mint_data.data.len()
    );

    // Step 2: Create ATAs for sender and recipient
    let recipient = Keypair::new();
    let source_ata = get_associated_token_address(&payer.pubkey(), &mint_keypair.pubkey());
    let dest_ata = get_associated_token_address(&recipient.pubkey(), &mint_keypair.pubkey());

    let create_source_ata_ix = create_associated_token_account_ix(
        &payer.pubkey(),
        &payer.pubkey(),
        &mint_keypair.pubkey(),
    );
    let create_dest_ata_ix = create_associated_token_account_ix(
        &payer.pubkey(),
        &recipient.pubkey(),
        &mint_keypair.pubkey(),
    );
    send_tx(
        &mut svm,
        &[create_source_ata_ix, create_dest_ata_ix],
        &payer,
        &[],
    );
    svm.expire_blockhash();

    // Step 3: Mint 1 token to sender
    let mint_ix = mint_to_ix(
        &mint_keypair.pubkey(),
        &source_ata,
        &payer.pubkey(),
        1,
    );
    send_tx(&mut svm, &[mint_ix], &payer, &[]);
    svm.expire_blockhash();

    // Step 4: Attempt transfer — should fail because mint is NonTransferable
    let transfer_ix = transfer_checked_ix(
        &source_ata,
        &mint_keypair.pubkey(),
        &dest_ata,
        &payer.pubkey(),
        1,
        2, // decimals
    );
    let result = try_send_tx(&mut svm, &[transfer_ix], &payer, &[]);
    assert!(
        result.is_err(),
        "Transfer should fail because the mint is non-transferable"
    );
}
