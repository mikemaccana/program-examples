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

fn memo_program_id() -> Pubkey {
    "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr"
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

/// Create a Token-2022 mint (no extensions) with given decimals.
fn create_mint_instructions(
    payer: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
    decimals: u8,
) -> Vec<Instruction> {
    let token_2022 = token_2022_program_id();
    let create_ix = anchor_lang::solana_program::system_instruction::create_account(
        payer,
        mint,
        10_000_000,
        82,
        &token_2022,
    );
    // InitializeMint2 (instruction 20): decimals, mint_authority, freeze_authority
    let mut data = vec![20u8, decimals];
    data.extend_from_slice(authority.as_ref());
    data.push(0); // no freeze authority
    let init_mint_ix = Instruction {
        program_id: token_2022,
        accounts: vec![AccountMeta::new(*mint, false)],
        data,
    };
    vec![create_ix, init_mint_ix]
}

/// Create a Token-2022 token account (165 bytes, no extra extensions).
fn create_token_account_instructions(
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

/// Transfer instruction (instruction 3).
fn transfer_instruction(
    source: &Pubkey,
    dest: &Pubkey,
    authority: &Pubkey,
    amount: u64,
) -> Instruction {
    let token_2022 = token_2022_program_id();
    let mut data = vec![3u8];
    data.extend_from_slice(&amount.to_le_bytes());
    Instruction {
        program_id: token_2022,
        accounts: vec![
            AccountMeta::new(*source, false),
            AccountMeta::new(*dest, false),
            AccountMeta::new_readonly(*authority, true),
        ],
        data,
    }
}

/// Memo instruction: just the memo text as bytes.
fn memo_instruction(memo_text: &str, signers: &[&Pubkey]) -> Instruction {
    let accounts: Vec<AccountMeta> = signers
        .iter()
        .map(|s| AccountMeta::new_readonly(**s, true))
        .collect();
    Instruction {
        program_id: memo_program_id(),
        accounts,
        data: memo_text.as_bytes().to_vec(),
    }
}

fn setup() -> (LiteSVM, Pubkey, Keypair) {
    let program_id = memo_transfer::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/memo_transfer.so");
    svm.add_program(program_id, program_bytes).unwrap();

    // Load SPL Memo program (needed for memo instructions)
    // Memo program .so sourced from litesvm's bundled ELF files
    let memo_program_bytes = include_bytes!("/tmp/so-backup/spl_memo.so");
    svm.add_program(memo_program_id(), memo_program_bytes)
        .unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, program_id, payer)
}

#[test]
fn test_memo_transfer() {
    let (mut svm, program_id, payer) = setup();
    let mint_keypair = Keypair::new();
    let token_keypair = Keypair::new();

    // Step 1: Create a standard Token-2022 mint
    let create_mint_ixs = create_mint_instructions(
        &payer.pubkey(),
        &mint_keypair.pubkey(),
        &payer.pubkey(),
        2,
    );
    send_tx(&mut svm, &create_mint_ixs, &payer, &[&mint_keypair]);
    svm.expire_blockhash();

    // Step 2: Create token account with RequiredMemo extension via program
    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &memo_transfer::instruction::Initialize {}.data(),
        memo_transfer::accounts::Initialize {
            payer: payer.pubkey(),
            token_account: token_keypair.pubkey(),
            mint_account: mint_keypair.pubkey(),
            token_program: token_2022_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, &[initialize_ix], &payer, &[&token_keypair]);

    // Verify token account exists
    let token_account = svm
        .get_account(&token_keypair.pubkey())
        .expect("Token account should exist");
    assert!(
        !token_account.data.is_empty(),
        "Token account should have data"
    );

    svm.expire_blockhash();

    // Step 3: Create a source token account and mint tokens to it
    let source_keypair = Keypair::new();
    let create_source_ixs = create_token_account_instructions(
        &payer.pubkey(),
        &source_keypair.pubkey(),
        &mint_keypair.pubkey(),
        &payer.pubkey(),
    );
    send_tx(&mut svm, &create_source_ixs, &payer, &[&source_keypair]);
    svm.expire_blockhash();

    let mint_to_ix = mint_to_instruction(
        &mint_keypair.pubkey(),
        &source_keypair.pubkey(),
        &payer.pubkey(),
        100,
    );
    send_tx(&mut svm, &[mint_to_ix], &payer, &[]);
    svm.expire_blockhash();

    // Step 4: Transfer without memo — should fail
    let transfer_ix = transfer_instruction(
        &source_keypair.pubkey(),
        &token_keypair.pubkey(),
        &payer.pubkey(),
        1,
    );
    let result = try_send_tx(&mut svm, &[transfer_ix], &payer, &[]);
    assert!(
        result.is_err(),
        "Transfer without memo should fail"
    );
    svm.expire_blockhash();

    // Step 5: Transfer with memo — should succeed
    let memo_ix = memo_instruction("hello, world", &[&payer.pubkey()]);
    let transfer_ix = transfer_instruction(
        &source_keypair.pubkey(),
        &token_keypair.pubkey(),
        &payer.pubkey(),
        1,
    );
    send_tx(&mut svm, &[memo_ix, transfer_ix], &payer, &[]);
    svm.expire_blockhash();

    // Verify transfer succeeded
    let dest_data = svm.get_account(&token_keypair.pubkey()).unwrap();
    let amount = u64::from_le_bytes(dest_data.data[64..72].try_into().unwrap());
    assert_eq!(amount, 1, "Should have 1 token after transfer with memo");

    // Step 6: Disable RequiredMemo extension
    let disable_ix = Instruction::new_with_bytes(
        program_id,
        &memo_transfer::instruction::Disable {}.data(),
        memo_transfer::accounts::Disable {
            owner: payer.pubkey(),
            token_account: token_keypair.pubkey(),
            token_program: token_2022_program_id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, &[disable_ix], &payer, &[]);
    svm.expire_blockhash();

    // Step 7: Transfer without memo should now succeed (memo disabled)
    // Create new source account with tokens
    let source2_keypair = Keypair::new();
    let create_source2_ixs = create_token_account_instructions(
        &payer.pubkey(),
        &source2_keypair.pubkey(),
        &mint_keypair.pubkey(),
        &payer.pubkey(),
    );
    send_tx(&mut svm, &create_source2_ixs, &payer, &[&source2_keypair]);
    svm.expire_blockhash();

    let mint_to_ix2 = mint_to_instruction(
        &mint_keypair.pubkey(),
        &source2_keypair.pubkey(),
        &payer.pubkey(),
        100,
    );
    send_tx(&mut svm, &[mint_to_ix2], &payer, &[]);
    svm.expire_blockhash();

    let transfer_ix2 = transfer_instruction(
        &source2_keypair.pubkey(),
        &token_keypair.pubkey(),
        &payer.pubkey(),
        1,
    );
    send_tx(&mut svm, &[transfer_ix2], &payer, &[]);

    // Verify transfer succeeded
    let dest_data = svm.get_account(&token_keypair.pubkey()).unwrap();
    let final_amount = u64::from_le_bytes(dest_data.data[64..72].try_into().unwrap());
    assert_eq!(
        final_amount, 2,
        "Should have 2 tokens after transfer without memo (memo disabled)"
    );
}
