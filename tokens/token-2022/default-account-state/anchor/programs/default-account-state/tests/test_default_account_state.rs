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

/// Create a Token-2022 token account (165 bytes, no extra extensions).
fn create_token_account_instruction(
    payer: &Pubkey,
    token_account: &Pubkey,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Vec<Instruction> {
    let token_2022 = token_2022_program_id();
    let rent_sysvar: Pubkey = "SysvarRent111111111111111111111111111111111"
        .parse()
        .unwrap();
    let create_ix = anchor_lang::solana_program::system_instruction::create_account(
        payer,
        token_account,
        3_000_000,
        165,
        &token_2022,
    );
    let init_ix = Instruction {
        program_id: token_2022,
        accounts: vec![
            AccountMeta::new(*token_account, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(*owner, false),
            AccountMeta::new_readonly(rent_sysvar, false),
        ],
        data: vec![1], // InitializeAccount
    };
    vec![create_ix, init_ix]
}

/// MintTo instruction (instruction 7).
fn mint_to_instruction(
    mint: &Pubkey,
    dest: &Pubkey,
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
            AccountMeta::new(*dest, false),
            AccountMeta::new_readonly(*authority, true),
        ],
        data,
    }
}

fn setup() -> (LiteSVM, Pubkey, Keypair) {
    let program_id = default_account_state::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/default_account_state.so");
    svm.add_program(program_id, program_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, program_id, payer)
}

#[test]
fn test_default_account_state() {
    let (mut svm, program_id, payer) = setup();
    let mint_keypair = Keypair::new();
    let token_2022 = token_2022_program_id();

    // Step 1: Initialize mint with DefaultAccountState extension (frozen)
    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &default_account_state::instruction::Initialize {}.data(),
        default_account_state::accounts::Initialize {
            payer: payer.pubkey(),
            mint_account: mint_keypair.pubkey(),
            token_program: token_2022,
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, &[initialize_ix], &payer, &[&mint_keypair]);
    svm.expire_blockhash();

    // Verify mint exists
    let mint_account = svm.get_account(&mint_keypair.pubkey()).unwrap();
    assert!(!mint_account.data.is_empty(), "Mint should have data");

    // Step 2: Create a token account (it will be frozen by default due to DefaultAccountState extension)
    let token1 = Keypair::new();
    let create_token1_ixs = create_token_account_instruction(
        &payer.pubkey(),
        &token1.pubkey(),
        &mint_keypair.pubkey(),
        &payer.pubkey(),
    );
    send_tx(&mut svm, &create_token1_ixs, &payer, &[&token1]);
    svm.expire_blockhash();

    // Verify token account state is frozen (byte 108 = account state: 0=uninitialized, 1=initialized, 2=frozen)
    let token1_data = svm.get_account(&token1.pubkey()).unwrap();
    assert_eq!(
        token1_data.data[108], 2,
        "Token account should be frozen (state=2)"
    );

    // Step 3: Attempt to mint to the frozen account — should fail
    let mint_to_ix = mint_to_instruction(
        &mint_keypair.pubkey(),
        &token1.pubkey(),
        &payer.pubkey(),
        1,
    );
    let result = try_send_tx(&mut svm, &[mint_to_ix], &payer, &[]);
    assert!(
        result.is_err(),
        "Minting to a frozen account should fail"
    );
    svm.expire_blockhash();

    // Step 4: Update default state to Initialized
    let update_ix = Instruction::new_with_bytes(
        program_id,
        &default_account_state::instruction::UpdateDefaultState {
            account_state: default_account_state::AnchorAccountState::Initialized,
        }
        .data(),
        default_account_state::accounts::UpdateDefaultState {
            freeze_authority: payer.pubkey(),
            mint_account: mint_keypair.pubkey(),
            token_program: token_2022,
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, &[update_ix], &payer, &[]);
    svm.expire_blockhash();

    // Step 5: Create a new token account — should be initialized (not frozen) now
    let token2 = Keypair::new();
    let create_token2_ixs = create_token_account_instruction(
        &payer.pubkey(),
        &token2.pubkey(),
        &mint_keypair.pubkey(),
        &payer.pubkey(),
    );
    send_tx(&mut svm, &create_token2_ixs, &payer, &[&token2]);
    svm.expire_blockhash();

    // Verify token2 is initialized (not frozen)
    let token2_data = svm.get_account(&token2.pubkey()).unwrap();
    assert_eq!(
        token2_data.data[108], 1,
        "Token account should be initialized (state=1)"
    );

    // Step 6: Mint to the new account — should succeed
    let mint_to_ix2 = mint_to_instruction(
        &mint_keypair.pubkey(),
        &token2.pubkey(),
        &payer.pubkey(),
        1,
    );
    send_tx(&mut svm, &[mint_to_ix2], &payer, &[]);

    // Verify tokens were minted
    let token2_data = svm.get_account(&token2.pubkey()).unwrap();
    let amount = u64::from_le_bytes(token2_data.data[64..72].try_into().unwrap());
    assert_eq!(amount, 1, "Should have minted 1 token");
}
