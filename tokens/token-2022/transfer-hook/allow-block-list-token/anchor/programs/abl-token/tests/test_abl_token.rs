use {
    anchor_lang::{
        solana_program::{
            instruction::Instruction,
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
    let program_id = abl_token::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/abl_token.so");
    svm.add_program(program_id, program_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, program_id, payer)
}

#[test]
fn test_init_config_and_init_mint() {
    let (mut svm, program_id, payer) = setup();
    let mint_keypair = Keypair::new();
    let token_2022 = token_2022_program_id();

    // Derive PDAs
    let (config_pda, _) =
        Pubkey::find_program_address(&[b"config"], &program_id);
    let (extra_account_meta_list, _) = Pubkey::find_program_address(
        &[b"extra-account-metas", mint_keypair.pubkey().as_ref()],
        &program_id,
    );

    // Step 1: Initialize config
    let init_config_ix = Instruction::new_with_bytes(
        program_id,
        &abl_token::instruction::InitConfig {}.data(),
        abl_token::accounts::InitConfig {
            payer: payer.pubkey(),
            config: config_pda,
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, &[init_config_ix], &payer, &[]);
    svm.expire_blockhash();

    // Step 2: Initialize mint with transfer hook and metadata
    let init_mint_args = abl_token::instructions::InitMintArgs {
        name: "Test Token".to_string(),
        symbol: "TEST".to_string(),
        uri: "https://test.com".to_string(),
        decimals: 6,
        mint_authority: payer.pubkey(),
        freeze_authority: payer.pubkey(),
        permanent_delegate: payer.pubkey(),
        transfer_hook_authority: payer.pubkey(),
        mode: abl_token::Mode::Allow,
        threshold: 0,
    };
    let init_mint_ix = Instruction::new_with_bytes(
        program_id,
        &abl_token::instruction::InitMint {
            args: init_mint_args,
        }
        .data(),
        abl_token::accounts::InitMint {
            payer: payer.pubkey(),
            mint: mint_keypair.pubkey(),
            extra_metas_account: extra_account_meta_list,
            system_program: system_program::id(),
            token_program: token_2022,
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, &[init_mint_ix], &payer, &[&mint_keypair]);
}
