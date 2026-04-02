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

fn setup() -> (LiteSVM, Pubkey, Keypair) {
    let program_id = transfer_hook::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/transfer_hook.so");
    svm.add_program(program_id, program_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, program_id, payer)
}

fn create_mint_with_transfer_hook_instructions(
    payer: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
    hook_program_id: &Pubkey,
    decimals: u8,
) -> Vec<Instruction> {
    let token_2022 = token_2022_program_id();
    let mint_space: u64 = 234;
    let lamports: u64 = 5_000_000;

    let create_account_ix = anchor_lang::solana_program::system_instruction::create_account(
        payer, mint, lamports, mint_space, &token_2022,
    );

    let mut init_hook_data = vec![36u8, 0u8];
    init_hook_data.extend_from_slice(authority.as_ref());
    init_hook_data.extend_from_slice(hook_program_id.as_ref());
    let init_hook_ix = Instruction {
        program_id: token_2022,
        accounts: vec![AccountMeta::new(*mint, false)],
        data: init_hook_data,
    };

    let mut init_mint_data = vec![0u8, decimals];
    init_mint_data.extend_from_slice(authority.as_ref());
    init_mint_data.push(0);
    init_mint_data.extend_from_slice(&[0u8; 32]);
    let init_mint_ix = Instruction {
        program_id: token_2022,
        accounts: vec![
            AccountMeta::new(*mint, false),
            AccountMeta::new_readonly(
                "SysvarRent111111111111111111111111111111111"
                    .parse()
                    .unwrap(),
                false,
            ),
        ],
        data: init_mint_data,
    };

    vec![create_account_ix, init_hook_ix, init_mint_ix]
}

/// Test that the program initializes the ExtraAccountMetaList and counter account.
/// The full transfer flow requires WSOL setup (base SPL Token program, wrapped SOL
/// accounts, approve + syncNative) which is tested by the TS integration test.
#[test]
fn test_initialize_extra_account_meta_list() {
    let (mut svm, program_id, payer) = setup();
    let mint_keypair = Keypair::new();
    let decimals: u8 = 9;

    // PDAs
    let (extra_account_meta_list, _) = Pubkey::find_program_address(
        &[b"extra-account-metas", mint_keypair.pubkey().as_ref()],
        &program_id,
    );
    let (counter_pda, _) = Pubkey::find_program_address(&[b"counter"], &program_id);

    // Step 1: Create mint with TransferHook extension
    let mint_ixs = create_mint_with_transfer_hook_instructions(
        &payer.pubkey(),
        &mint_keypair.pubkey(),
        &payer.pubkey(),
        &program_id,
        decimals,
    );
    send_tx(&mut svm, &mint_ixs, &payer, &[&mint_keypair]);
    svm.expire_blockhash();

    // Step 2: Initialize ExtraAccountMetaList (also creates counter PDA)
    let init_extra_ix = Instruction::new_with_bytes(
        program_id,
        &transfer_hook::instruction::InitializeExtraAccountMetaList {}.data(),
        transfer_hook::accounts::InitializeExtraAccountMetaList {
            payer: payer.pubkey(),
            extra_account_meta_list,
            mint: mint_keypair.pubkey(),
            counter_account: counter_pda,
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, &[init_extra_ix], &payer, &[]);

    // Verify the ExtraAccountMetaList account was created
    let account = svm.get_account(&extra_account_meta_list);
    assert!(
        account.is_some(),
        "ExtraAccountMetaList account should exist after initialization"
    );

    // Verify the counter account was created
    let counter = svm.get_account(&counter_pda);
    assert!(
        counter.is_some(),
        "Counter account should exist after initialization"
    );
}
