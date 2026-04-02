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
    let program_id = permanent_delegate::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/permanent_delegate.so");
    svm.add_program(program_id, program_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, program_id, payer)
}

/// Create a Token-2022 token account (CreateAccount + InitializeAccount3 = instruction 18).
fn create_token_account_instructions(
    payer: &Pubkey,
    account: &Pubkey,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Vec<Instruction> {
    let token_2022 = token_2022_program_id();
    // Token-2022 token accounts may need extra space for extensions (TransferHookAccount, etc.)
    // Use a generous allocation to cover any auto-added extensions.
    let space: u64 = 200;
    let lamports: u64 = 3_000_000; // enough for rent exemption
    let create_account_ix = anchor_lang::solana_program::system_instruction::create_account(
        payer, account, lamports, space, &token_2022,
    );
    // InitializeAccount3 (instruction 18): no separate rent sysvar needed
    let mut init_data = vec![18u8];
    init_data.extend_from_slice(owner.as_ref());
    let init_account_ix = Instruction {
        program_id: token_2022,
        accounts: vec![
            AccountMeta::new(*account, false),
            AccountMeta::new_readonly(*mint, false),
        ],
        data: init_data,
    };
    vec![create_account_ix, init_account_ix]
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

/// BurnChecked instruction for Token-2022 (instruction 15).
fn burn_checked_ix(
    account: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
    amount: u64,
    decimals: u8,
) -> Instruction {
    let token_2022 = token_2022_program_id();
    let mut data = vec![15u8];
    data.extend_from_slice(&amount.to_le_bytes());
    data.push(decimals);
    Instruction {
        program_id: token_2022,
        accounts: vec![
            AccountMeta::new(*account, false),
            AccountMeta::new(*mint, false),
            AccountMeta::new_readonly(*authority, true),
        ],
        data,
    }
}

#[test]
fn test_create_mint_with_permanent_delegate_and_burn() {
    let (mut svm, program_id, payer) = setup();
    let mint_keypair = Keypair::new();
    let token_2022 = token_2022_program_id();

    // Step 1: Create mint with PermanentDelegate extension via program
    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &permanent_delegate::instruction::Initialize {}.data(),
        permanent_delegate::accounts::Initialize {
            payer: payer.pubkey(),
            mint_account: mint_keypair.pubkey(),
            token_program: token_2022,
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, &[initialize_ix], &payer, &[&mint_keypair]);
    svm.expire_blockhash();

    // Step 2: Create a token account owned by a random keypair
    let random_owner = Keypair::new();
    let token_account = Keypair::new();
    let create_ata_ixs = create_token_account_instructions(
        &payer.pubkey(),
        &token_account.pubkey(),
        &mint_keypair.pubkey(),
        &random_owner.pubkey(),
    );
    send_tx(&mut svm, &create_ata_ixs, &payer, &[&token_account]);
    svm.expire_blockhash();

    // Step 3: Mint 100 tokens to the token account
    let mint_ix = mint_to_ix(
        &mint_keypair.pubkey(),
        &token_account.pubkey(),
        &payer.pubkey(),
        100,
    );
    send_tx(&mut svm, &[mint_ix], &payer, &[]);
    svm.expire_blockhash();

    // Step 4: Burn all 100 tokens using the permanent delegate (payer)
    // The permanent delegate can burn tokens from any token account for this mint,
    // even though payer is not the token account owner.
    let burn_ix = burn_checked_ix(
        &token_account.pubkey(),
        &mint_keypair.pubkey(),
        &payer.pubkey(),
        100,
        2, // decimals
    );
    send_tx(&mut svm, &[burn_ix], &payer, &[]);

    // Verify token account balance is 0
    let account_data = svm
        .get_account(&token_account.pubkey())
        .expect("Token account should exist");
    // Token account data: offset 64 is the amount field (u64 LE)
    let amount = u64::from_le_bytes(account_data.data[64..72].try_into().unwrap());
    assert_eq!(amount, 0, "Token account balance should be 0 after burn");
}
