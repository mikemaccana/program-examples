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

fn setup() -> (LiteSVM, Pubkey, Keypair) {
    let program_id = cpi_guard::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/cpi_guard.so");
    svm.add_program(program_id, program_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, program_id, payer)
}

/// Create Token-2022 mint with given decimals (InitializeMint2 = instruction 20).
fn create_mint_instructions(
    payer: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
    decimals: u8,
) -> Vec<Instruction> {
    let token_2022 = token_2022_program_id();
    let create_account_ix = anchor_lang::solana_program::system_instruction::create_account(
        payer, mint, 2_000_000, 82, &token_2022,
    );
    let mut init_data = vec![20u8, decimals];
    init_data.extend_from_slice(authority.as_ref());
    init_data.push(0); // no freeze authority
    init_data.extend_from_slice(&[0u8; 32]);
    let init_mint_ix = Instruction {
        program_id: token_2022,
        accounts: vec![AccountMeta::new(*mint, false)],
        data: init_data,
    };
    vec![create_account_ix, init_mint_ix]
}

/// Create a basic Token-2022 token account (165 bytes, no extensions).
fn create_basic_token_account_instructions(
    payer: &Pubkey,
    token_account: &Pubkey,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Vec<Instruction> {
    let token_2022 = token_2022_program_id();
    let rent_sysvar: Pubkey = "SysvarRent111111111111111111111111111111111"
        .parse()
        .unwrap();
    let create_account_ix = anchor_lang::solana_program::system_instruction::create_account(
        payer,
        token_account,
        3_000_000,
        165,
        &token_2022,
    );
    let init_account_ix = Instruction {
        program_id: token_2022,
        accounts: vec![
            AccountMeta::new(*token_account, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(*owner, false),
            AccountMeta::new_readonly(rent_sysvar, false),
        ],
        data: vec![1], // InitializeAccount
    };
    vec![create_account_ix, init_account_ix]
}

/// Reallocate instruction (instruction 29) to add extension types to a token account.
/// The system program and payer need to pay for additional rent.
fn reallocate_instruction(
    token_account: &Pubkey,
    payer: &Pubkey,
    owner: &Pubkey,
    extension_types: &[u16],
) -> Instruction {
    let token_2022 = token_2022_program_id();
    let mut data = vec![29u8];
    for et in extension_types {
        data.extend_from_slice(&et.to_le_bytes());
    }
    Instruction {
        program_id: token_2022,
        accounts: vec![
            AccountMeta::new(*token_account, false),
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(*owner, true),
        ],
        data,
    }
}

/// EnableCpiGuard instruction (instruction 34, sub-instruction 0).
fn enable_cpi_guard_instruction(token_account: &Pubkey, owner: &Pubkey) -> Instruction {
    let token_2022 = token_2022_program_id();
    Instruction {
        program_id: token_2022,
        accounts: vec![
            AccountMeta::new(*token_account, false),
            AccountMeta::new_readonly(*owner, true),
        ],
        data: vec![34, 0],
    }
}

/// DisableCpiGuard instruction (instruction 34, sub-instruction 1).
fn disable_cpi_guard_instruction(token_account: &Pubkey, owner: &Pubkey) -> Instruction {
    let token_2022 = token_2022_program_id();
    Instruction {
        program_id: token_2022,
        accounts: vec![
            AccountMeta::new(*token_account, false),
            AccountMeta::new_readonly(*owner, true),
        ],
        data: vec![34, 1],
    }
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

#[test]
fn test_cpi_guard_prevents_transfer_then_allows_after_disable() {
    let (mut svm, program_id, payer) = setup();
    let mint_keypair = Keypair::new();
    let token_keypair = Keypair::new();

    // Step 1: Create a Token-2022 mint
    let mint_ixs = create_mint_instructions(
        &payer.pubkey(),
        &mint_keypair.pubkey(),
        &payer.pubkey(),
        2,
    );
    send_tx(&mut svm, &mint_ixs, &payer, &[&mint_keypair]);
    svm.expire_blockhash();

    // Step 2: Create basic token account
    let token_ixs = create_basic_token_account_instructions(
        &payer.pubkey(),
        &token_keypair.pubkey(),
        &mint_keypair.pubkey(),
        &payer.pubkey(),
    );
    send_tx(&mut svm, &token_ixs, &payer, &[&token_keypair]);
    svm.expire_blockhash();

    // Step 3: Reallocate to add CPI Guard extension space
    let cpi_guard_extension_type: u16 = 11; // ExtensionType::CpiGuard
    let reallocate_ix = reallocate_instruction(
        &token_keypair.pubkey(),
        &payer.pubkey(),
        &payer.pubkey(),
        &[cpi_guard_extension_type],
    );
    send_tx(&mut svm, &[reallocate_ix], &payer, &[]);
    svm.expire_blockhash();

    // Step 4: Enable CPI Guard
    let enable_ix = enable_cpi_guard_instruction(&token_keypair.pubkey(), &payer.pubkey());
    send_tx(&mut svm, &[enable_ix], &payer, &[]);
    svm.expire_blockhash();

    // Step 5: Mint 1 token to the token account
    let mint_to_ix = mint_to_instruction(
        &mint_keypair.pubkey(),
        &token_keypair.pubkey(),
        &payer.pubkey(),
        1,
    );
    send_tx(&mut svm, &[mint_to_ix], &payer, &[]);
    svm.expire_blockhash();

    // Step 6: Try CPI transfer — should fail because CPI Guard is enabled
    let (recipient_token_account, _bump) =
        Pubkey::find_program_address(&[b"pda"], &program_id);

    let transfer_ix = Instruction::new_with_bytes(
        program_id,
        &cpi_guard::instruction::CpiTransfer {}.data(),
        cpi_guard::accounts::CpiTransfer {
            sender: payer.pubkey(),
            sender_token_account: token_keypair.pubkey(),
            recipient_token_account,
            mint_account: mint_keypair.pubkey(),
            token_program: token_2022_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );

    let result = try_send_tx(&mut svm, &[transfer_ix], &payer, &[]);
    assert!(
        result.is_err(),
        "Transfer should fail when CPI Guard is enabled"
    );
    svm.expire_blockhash();

    // Step 7: Disable CPI Guard
    let disable_ix = disable_cpi_guard_instruction(&token_keypair.pubkey(), &payer.pubkey());
    send_tx(&mut svm, &[disable_ix], &payer, &[]);
    svm.expire_blockhash();

    // Step 8: Transfer should now succeed
    let transfer_ix2 = Instruction::new_with_bytes(
        program_id,
        &cpi_guard::instruction::CpiTransfer {}.data(),
        cpi_guard::accounts::CpiTransfer {
            sender: payer.pubkey(),
            sender_token_account: token_keypair.pubkey(),
            recipient_token_account,
            mint_account: mint_keypair.pubkey(),
            token_program: token_2022_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );

    send_tx(&mut svm, &[transfer_ix2], &payer, &[]);

    // Verify recipient got the token
    let recipient_data = svm
        .get_account(&recipient_token_account)
        .expect("Recipient token account should exist");
    let amount = u64::from_le_bytes(recipient_data.data[64..72].try_into().unwrap());
    assert_eq!(amount, 1, "Recipient should have 1 token");
}
