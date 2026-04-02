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

fn setup() -> (LiteSVM, Pubkey, Keypair) {
    let program_id = transfer_fee::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/transfer_fee.so");
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

/// MintTo instruction for Token-2022 (instruction 7).
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

#[test]
fn test_transfer_fee_full_flow() {
    let (mut svm, program_id, payer) = setup();
    let mint_keypair = Keypair::new();
    let recipient = Keypair::new();
    let token_2022 = token_2022_program_id();
    let ata_program = associated_token_program_id();

    let sender_ata = get_associated_token_address(&payer.pubkey(), &mint_keypair.pubkey());
    let recipient_ata = get_associated_token_address(&recipient.pubkey(), &mint_keypair.pubkey());

    // Step 1: Create mint with transfer fee (100 basis points = 1%, max fee = 1)
    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &transfer_fee::instruction::Initialize {
            transfer_fee_basis_points: 100,
            maximum_fee: 1,
        }
        .data(),
        transfer_fee::accounts::Initialize {
            payer: payer.pubkey(),
            mint_account: mint_keypair.pubkey(),
            token_program: token_2022,
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, &[initialize_ix], &payer, &[&mint_keypair]);
    svm.expire_blockhash();

    // Step 2: Create sender ATA and mint 300 tokens
    let create_sender_ata = create_associated_token_account_ix(
        &payer.pubkey(),
        &payer.pubkey(),
        &mint_keypair.pubkey(),
    );
    send_tx(&mut svm, &[create_sender_ata], &payer, &[]);
    svm.expire_blockhash();

    let mint_ix = mint_to_ix(&mint_keypair.pubkey(), &sender_ata, &payer.pubkey(), 300);
    send_tx(&mut svm, &[mint_ix], &payer, &[]);
    svm.expire_blockhash();

    // Step 3: Transfer 100 tokens (fee = min(1% * 100 = 1, max_fee = 1) = 1)
    let transfer_ix = Instruction::new_with_bytes(
        program_id,
        &transfer_fee::instruction::Transfer { amount: 100 }.data(),
        transfer_fee::accounts::Transfer {
            sender: payer.pubkey(),
            recipient: recipient.pubkey(),
            mint_account: mint_keypair.pubkey(),
            sender_token_account: sender_ata,
            recipient_token_account: recipient_ata,
            token_program: token_2022,
            associated_token_program: ata_program,
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, &[transfer_ix], &payer, &[]);
    svm.expire_blockhash();

    // Step 4: Transfer 200 tokens (fee = min(1% * 200 = 2, max_fee = 1) = 1, capped by maximumFee)
    let transfer_ix2 = Instruction::new_with_bytes(
        program_id,
        &transfer_fee::instruction::Transfer { amount: 200 }.data(),
        transfer_fee::accounts::Transfer {
            sender: payer.pubkey(),
            recipient: recipient.pubkey(),
            mint_account: mint_keypair.pubkey(),
            sender_token_account: sender_ata,
            recipient_token_account: recipient_ata,
            token_program: token_2022,
            associated_token_program: ata_program,
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, &[transfer_ix2], &payer, &[]);
    svm.expire_blockhash();

    // Step 5: Harvest transfer fees from recipient token account to mint
    let harvest_ix = Instruction::new_with_bytes(
        program_id,
        &transfer_fee::instruction::Harvest {}.data(),
        {
            let mut metas = transfer_fee::accounts::Harvest {
                mint_account: mint_keypair.pubkey(),
                token_program: token_2022,
            }
            .to_account_metas(None);
            // Add remaining account: the recipient token account to harvest from
            metas.push(AccountMeta::new(recipient_ata, false));
            metas
        },
    );
    send_tx(&mut svm, &[harvest_ix], &payer, &[]);
    svm.expire_blockhash();

    // Step 6: Withdraw harvested fees from mint to sender's token account
    let withdraw_ix = Instruction::new_with_bytes(
        program_id,
        &transfer_fee::instruction::Withdraw {}.data(),
        transfer_fee::accounts::Withdraw {
            authority: payer.pubkey(),
            mint_account: mint_keypair.pubkey(),
            token_account: sender_ata,
            token_program: token_2022,
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, &[withdraw_ix], &payer, &[]);
    svm.expire_blockhash();

    // Step 7: Update transfer fee to 0
    let update_fee_ix = Instruction::new_with_bytes(
        program_id,
        &transfer_fee::instruction::UpdateFee {
            transfer_fee_basis_points: 0,
            maximum_fee: 0,
        }
        .data(),
        transfer_fee::accounts::UpdateFee {
            authority: payer.pubkey(),
            mint_account: mint_keypair.pubkey(),
            token_program: token_2022,
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, &[update_fee_ix], &payer, &[]);
}
