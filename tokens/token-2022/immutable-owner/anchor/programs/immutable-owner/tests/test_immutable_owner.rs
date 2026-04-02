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

/// Create a Token-2022 mint (InitializeMint2 = instruction 20).
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

/// SetAuthority instruction for Token-2022 (instruction 6).
/// AuthorityType::AccountOwner = 2
fn set_authority_instruction(
    account: &Pubkey,
    current_authority: &Pubkey,
    new_authority: Option<&Pubkey>,
    authority_type: u8,
) -> Instruction {
    let token_2022 = token_2022_program_id();
    let mut data = vec![6u8, authority_type];
    match new_authority {
        Some(new_auth) => {
            data.push(1); // COption::Some
            data.extend_from_slice(new_auth.as_ref());
        }
        None => {
            data.push(0); // COption::None
            data.extend_from_slice(&[0u8; 32]);
        }
    }
    Instruction {
        program_id: token_2022,
        accounts: vec![
            AccountMeta::new(*account, false),
            AccountMeta::new_readonly(*current_authority, true),
        ],
        data,
    }
}

fn setup() -> (LiteSVM, Pubkey, Keypair) {
    let program_id = immutable_owner::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/immutable_owner.so");
    svm.add_program(program_id, program_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, program_id, payer)
}

#[test]
fn test_create_token_account_with_immutable_owner() {
    let (mut svm, program_id, payer) = setup();
    let mint_keypair = Keypair::new();
    let token_keypair = Keypair::new();
    let token_2022 = token_2022_program_id();

    // Step 1: Create a Token-2022 mint with 2 decimals
    let mint_ixs = create_mint_instructions(
        &payer.pubkey(),
        &mint_keypair.pubkey(),
        &payer.pubkey(),
        2,
    );
    send_tx(&mut svm, &mint_ixs, &payer, &[&mint_keypair]);
    svm.expire_blockhash();

    // Step 2: Call program to create token account with ImmutableOwner extension
    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &immutable_owner::instruction::Initialize {}.data(),
        immutable_owner::accounts::Initialize {
            payer: payer.pubkey(),
            token_account: token_keypair.pubkey(),
            mint_account: mint_keypair.pubkey(),
            token_program: token_2022,
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, &[initialize_ix], &payer, &[&token_keypair]);
    svm.expire_blockhash();

    // Verify token account was created
    let token_data = svm
        .get_account(&token_keypair.pubkey())
        .expect("Token account should exist");
    assert!(
        token_data.data.len() > 165,
        "Token account should have extension data (size > 165, got {})",
        token_data.data.len()
    );

    // Step 3: Attempt to change the account owner — should fail due to immutable owner
    let new_owner = Keypair::new();
    let set_authority_ix = set_authority_instruction(
        &token_keypair.pubkey(),
        &payer.pubkey(),
        Some(&new_owner.pubkey()),
        2, // AuthorityType::AccountOwner
    );
    let result = try_send_tx(&mut svm, &[set_authority_ix], &payer, &[]);
    assert!(
        result.is_err(),
        "Setting a new owner should fail due to ImmutableOwner extension"
    );
}
