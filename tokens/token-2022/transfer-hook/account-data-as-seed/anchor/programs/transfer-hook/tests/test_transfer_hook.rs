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
    let program_id = transfer_hook::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/transfer_hook.so");
    svm.add_program(program_id, program_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, program_id, payer)
}

fn get_associated_token_address(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    let token_2022 = token_2022_program_id();
    let ata_program = associated_token_program_id();
    let (address, _) = Pubkey::find_program_address(
        &[wallet.as_ref(), token_2022.as_ref(), mint.as_ref()],
        &ata_program,
    );
    address
}

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

fn mint_to_ix(mint: &Pubkey, dest: &Pubkey, authority: &Pubkey, amount: u64) -> Instruction {
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

/// TransferChecked with hook accounts for account-data-as-seed.
/// Extra accounts appended:
///   - ExtraAccountMetaList PDA (readonly)
///   - Counter PDA derived from [b"counter", owner_pubkey] (writable)
///   - Transfer hook program ID (readonly)
fn transfer_checked_with_hook_ix(
    source: &Pubkey,
    mint: &Pubkey,
    destination: &Pubkey,
    authority: &Pubkey,
    amount: u64,
    decimals: u8,
    extra_account_meta_list: &Pubkey,
    counter_pda: &Pubkey,
    hook_program: &Pubkey,
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
            // Transfer hook validation accounts
            AccountMeta::new_readonly(*extra_account_meta_list, false),
            AccountMeta::new(*counter_pda, false),
            AccountMeta::new_readonly(*hook_program, false),
        ],
        data,
    }
}

#[test]
fn test_transfer_hook_account_data_as_seed() {
    let (mut svm, program_id, payer) = setup();
    let mint_keypair = Keypair::new();
    let recipient = Keypair::new();
    let token_2022 = token_2022_program_id();
    let ata_program = associated_token_program_id();
    let decimals: u8 = 9;

    let source_ata = get_associated_token_address(&payer.pubkey(), &mint_keypair.pubkey());
    let dest_ata = get_associated_token_address(&recipient.pubkey(), &mint_keypair.pubkey());

    // PDAs
    let (extra_account_meta_list, _) = Pubkey::find_program_address(
        &[b"extra-account-metas", mint_keypair.pubkey().as_ref()],
        &program_id,
    );
    // Counter PDA uses owner's pubkey as seed (account data as seed pattern)
    let (counter_pda, _) =
        Pubkey::find_program_address(&[b"counter", payer.pubkey().as_ref()], &program_id);

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

    // Step 2: Create token accounts and mint tokens
    let amount: u64 = 100 * 10u64.pow(decimals as u32);
    let create_source = create_associated_token_account_ix(
        &payer.pubkey(),
        &payer.pubkey(),
        &mint_keypair.pubkey(),
    );
    let create_dest = create_associated_token_account_ix(
        &payer.pubkey(),
        &recipient.pubkey(),
        &mint_keypair.pubkey(),
    );
    let mint_ix = mint_to_ix(
        &mint_keypair.pubkey(),
        &source_ata,
        &payer.pubkey(),
        amount,
    );
    send_tx(&mut svm, &[create_source, create_dest, mint_ix], &payer, &[]);
    svm.expire_blockhash();

    // Step 3: Initialize ExtraAccountMetaList (also creates counter PDA)
    let init_extra_ix = Instruction::new_with_bytes(
        program_id,
        &transfer_hook::instruction::InitializeExtraAccountMetaList {}.data(),
        transfer_hook::accounts::InitializeExtraAccountMetaList {
            payer: payer.pubkey(),
            extra_account_meta_list,
            mint: mint_keypair.pubkey(),
            counter_account: counter_pda,
            token_program: token_2022,
            associated_token_program: ata_program,
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, &[init_extra_ix], &payer, &[]);
    svm.expire_blockhash();

    // Step 4: Transfer with hook — counter PDA derived from owner's data in source token account
    let transfer_amount: u64 = 1 * 10u64.pow(decimals as u32);
    let transfer_ix = transfer_checked_with_hook_ix(
        &source_ata,
        &mint_keypair.pubkey(),
        &dest_ata,
        &payer.pubkey(),
        transfer_amount,
        decimals,
        &extra_account_meta_list,
        &counter_pda,
        &program_id,
    );
    send_tx(&mut svm, &[transfer_ix], &payer, &[]);
    svm.expire_blockhash();

    // Step 5: Try calling transfer_hook directly (should fail — not transferring)
    let direct_hook_ix = Instruction::new_with_bytes(
        program_id,
        &transfer_hook::instruction::TransferHook { amount: 1 }.data(),
        transfer_hook::accounts::TransferHook {
            source_token: source_ata,
            mint: mint_keypair.pubkey(),
            destination_token: dest_ata,
            owner: payer.pubkey(),
            extra_account_meta_list,
            counter_account: counter_pda,
        }
        .to_account_metas(None),
    );
    let result = try_send_tx(&mut svm, &[direct_hook_ix], &payer, &[]);
    assert!(
        result.is_err(),
        "Calling transfer_hook directly should fail because token is not transferring"
    );
}
