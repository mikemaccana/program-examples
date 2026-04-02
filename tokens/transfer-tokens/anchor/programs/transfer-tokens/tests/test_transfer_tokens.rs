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

fn metadata_program_id() -> Pubkey {
    "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
        .parse()
        .unwrap()
}

fn token_program_id() -> Pubkey {
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        .parse()
        .unwrap()
}

fn associated_token_program_id() -> Pubkey {
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        .parse()
        .unwrap()
}

fn rent_sysvar_id() -> Pubkey {
    "SysvarRent111111111111111111111111111111111"
        .parse()
        .unwrap()
}

fn derive_metadata_pda(mint: &Pubkey) -> Pubkey {
    let metadata_pid = metadata_program_id();
    let (pda, _bump) = Pubkey::find_program_address(
        &[b"metadata", metadata_pid.as_ref(), mint.as_ref()],
        &metadata_pid,
    );
    pda
}

/// Derive the associated token account address (same logic as spl_associated_token_account).
fn derive_ata(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    let (ata, _bump) = Pubkey::find_program_address(
        &[
            wallet.as_ref(),
            token_program_id().as_ref(),
            mint.as_ref(),
        ],
        &associated_token_program_id(),
    );
    ata
}

/// Read the token amount from a SPL token account's raw data.
/// SPL Token Account layout: mint(32) + owner(32) + amount(8) + ...
fn read_token_amount(data: &[u8]) -> u64 {
    let amount_bytes: [u8; 8] = data[64..72].try_into().unwrap();
    u64::from_le_bytes(amount_bytes)
}

fn setup() -> (LiteSVM, Pubkey, Keypair) {
    let program_id = transfer_tokens::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/transfer_tokens.so");
    svm.add_program(program_id, program_bytes).unwrap();

    let metadata_bytes = include_bytes!("../../../tests/fixtures/mpl_token_metadata.so");
    svm.add_program(metadata_program_id(), metadata_bytes)
        .unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, program_id, payer)
}

fn send_tx(
    svm: &mut LiteSVM,
    instruction: Instruction,
    payer: &Keypair,
    extra_signers: &[&Keypair],
) {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);
    let mut signers: Vec<&Keypair> = vec![payer];
    signers.extend_from_slice(extra_signers);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &signers).unwrap();
    svm.send_transaction(tx).unwrap();
}

#[test]
fn test_create_mint_and_transfer() {
    let (mut svm, program_id, payer) = setup();
    let mint_keypair = Keypair::new();
    let metadata_account = derive_metadata_pda(&mint_keypair.pubkey());

    // 1. Create token (with metadata)
    let create_ix = Instruction::new_with_bytes(
        program_id,
        &transfer_tokens::instruction::CreateToken {
            token_title: "Solana Gold".to_string(),
            token_symbol: "GOLDSOL".to_string(),
            token_uri: "https://example.com/token.json".to_string(),
        }
        .data(),
        transfer_tokens::accounts::CreateToken {
            payer: payer.pubkey(),
            mint_account: mint_keypair.pubkey(),
            metadata_account,
            token_program: token_program_id(),
            token_metadata_program: metadata_program_id(),
            system_program: system_program::id(),
            rent: rent_sysvar_id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, create_ix, &payer, &[&mint_keypair]);

    // Verify mint created
    let mint_account = svm
        .get_account(&mint_keypair.pubkey())
        .expect("Mint should exist");
    assert!(!mint_account.data.is_empty());

    // 2. Mint tokens (100 tokens to payer's ATA)
    svm.expire_blockhash();
    let sender_ata = derive_ata(&payer.pubkey(), &mint_keypair.pubkey());

    let mint_ix = Instruction::new_with_bytes(
        program_id,
        &transfer_tokens::instruction::MintToken { amount: 100 }.data(),
        transfer_tokens::accounts::MintToken {
            mint_authority: payer.pubkey(),
            recipient: payer.pubkey(),
            mint_account: mint_keypair.pubkey(),
            associated_token_account: sender_ata,
            token_program: token_program_id(),
            associated_token_program: associated_token_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, mint_ix, &payer, &[]);

    // Verify tokens minted — 100 * 10^9 = 100_000_000_000 (9 decimals)
    let sender_account_data = svm.get_account(&sender_ata).expect("Sender ATA should exist");
    assert_eq!(read_token_amount(&sender_account_data.data), 100_000_000_000);

    // 3. Transfer tokens (50 tokens to recipient)
    svm.expire_blockhash();
    let recipient = Keypair::new();
    let recipient_ata = derive_ata(&recipient.pubkey(), &mint_keypair.pubkey());

    let transfer_ix = Instruction::new_with_bytes(
        program_id,
        &transfer_tokens::instruction::TransferTokens { amount: 50 }.data(),
        transfer_tokens::accounts::TransferTokens {
            sender: payer.pubkey(),
            recipient: recipient.pubkey(),
            mint_account: mint_keypair.pubkey(),
            sender_token_account: sender_ata,
            recipient_token_account: recipient_ata,
            token_program: token_program_id(),
            associated_token_program: associated_token_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, transfer_ix, &payer, &[]);

    // Verify: sender 50 tokens, recipient 50 tokens (at 9 decimals)
    let sender_data = svm.get_account(&sender_ata).unwrap();
    assert_eq!(read_token_amount(&sender_data.data), 50_000_000_000);

    let recipient_data = svm.get_account(&recipient_ata).unwrap();
    assert_eq!(read_token_amount(&recipient_data.data), 50_000_000_000);
}
