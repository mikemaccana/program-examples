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

fn token_program_id() -> Pubkey {
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        .parse()
        .unwrap()
}
fn ata_program_id() -> Pubkey {
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        .parse()
        .unwrap()
}

/// Derive ATA address manually.
fn derive_ata(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    let (ata, _bump) = Pubkey::find_program_address(
        &[wallet.as_ref(), token_program_id().as_ref(), mint.as_ref()],
        &ata_program_id(),
    );
    ata
}

/// Read the token amount from raw SPL Token Account data (offset 64..72).
fn read_token_amount(data: &[u8]) -> u64 {
    u64::from_le_bytes(data[64..72].try_into().unwrap())
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
    let program_id = external_delegate_token_master::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/external_delegate_token_master.so");
    svm.add_program(program_id, program_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, program_id, payer)
}

/// Create an SPL mint and return its keypair.
fn create_mint(svm: &mut LiteSVM, payer: &Keypair, decimals: u8) -> Keypair {
    let mint_keypair = Keypair::new();
    let rent_lamports = 1_461_600;
    let create_ix = anchor_lang::solana_program::system_instruction::create_account(
        &payer.pubkey(),
        &mint_keypair.pubkey(),
        rent_lamports,
        82,
        &token_program_id(),
    );

    let mut init_data = vec![20u8]; // InitializeMint2
    init_data.push(decimals);
    init_data.extend_from_slice(payer.pubkey().as_ref());
    init_data.push(1);
    init_data.extend_from_slice(payer.pubkey().as_ref());

    let init_ix = Instruction {
        program_id: token_program_id(),
        accounts: vec![anchor_lang::solana_program::instruction::AccountMeta::new(
            mint_keypair.pubkey(),
            false,
        )],
        data: init_data,
    };

    send_tx(svm, &[create_ix, init_ix], payer, &[&mint_keypair]);
    mint_keypair
}

/// Create an ATA for a wallet.
fn create_ata_ix(payer: &Pubkey, wallet: &Pubkey, mint: &Pubkey) -> Instruction {
    let ata = derive_ata(wallet, mint);
    Instruction {
        program_id: ata_program_id(),
        accounts: vec![
            anchor_lang::solana_program::instruction::AccountMeta::new(*payer, true),
            anchor_lang::solana_program::instruction::AccountMeta::new(ata, false),
            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(*wallet, false),
            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(*mint, false),
            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                system_program::id(),
                false,
            ),
            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                token_program_id(),
                false,
            ),
        ],
        data: vec![0],
    }
}

/// Build a MintTo instruction.
fn mint_to_ix(mint: &Pubkey, dest: &Pubkey, authority: &Pubkey, amount: u64) -> Instruction {
    let mut data = vec![7u8];
    data.extend_from_slice(&amount.to_le_bytes());
    Instruction {
        program_id: token_program_id(),
        accounts: vec![
            anchor_lang::solana_program::instruction::AccountMeta::new(*mint, false),
            anchor_lang::solana_program::instruction::AccountMeta::new(*dest, false),
            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(*authority, true),
        ],
        data,
    }
}

#[test]
fn test_initialize_user_account() {
    let (mut svm, program_id, authority) = setup();
    let user_account = Keypair::new();

    let init_ix = Instruction::new_with_bytes(
        program_id,
        &external_delegate_token_master::instruction::Initialize {}.data(),
        external_delegate_token_master::accounts::Initialize {
            user_account: user_account.pubkey(),
            authority: authority.pubkey(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );

    send_tx(&mut svm, &[init_ix], &authority, &[&user_account]);

    // Verify the account was created
    let account_data = svm
        .get_account(&user_account.pubkey())
        .expect("User account should exist");

    // Skip 8-byte discriminator
    let data = &account_data.data[8..];
    // authority: Pubkey (32 bytes)
    let stored_authority = Pubkey::try_from(&data[0..32]).unwrap();
    assert_eq!(stored_authority, authority.pubkey());

    // ethereum_address: [u8; 20] — should be all zeros
    let eth_addr = &data[32..52];
    assert_eq!(eth_addr, &[0u8; 20]);
}

#[test]
fn test_set_ethereum_address() {
    let (mut svm, program_id, authority) = setup();
    let user_account = Keypair::new();

    // Initialize
    let init_ix = Instruction::new_with_bytes(
        program_id,
        &external_delegate_token_master::instruction::Initialize {}.data(),
        external_delegate_token_master::accounts::Initialize {
            user_account: user_account.pubkey(),
            authority: authority.pubkey(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, &[init_ix], &authority, &[&user_account]);

    // Set ethereum address
    svm.expire_blockhash();
    let ethereum_address: [u8; 20] = [
        0x1C, 0x8c, 0xd0, 0xc3, 0x8F, 0x8D, 0xE3, 0x5d, 0x60, 0x56, 0xc7, 0xC7, 0xaB, 0xFa,
        0x7e, 0x65, 0xD2, 0x60, 0xE8, 0x16,
    ];

    let set_eth_ix = Instruction::new_with_bytes(
        program_id,
        &external_delegate_token_master::instruction::SetEthereumAddress {
            ethereum_address,
        }
        .data(),
        external_delegate_token_master::accounts::SetEthereumAddress {
            user_account: user_account.pubkey(),
            authority: authority.pubkey(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, &[set_eth_ix], &authority, &[]);

    // Verify
    let account_data = svm
        .get_account(&user_account.pubkey())
        .expect("User account should exist");
    let data = &account_data.data[8..];
    let stored_eth_addr = &data[32..52];
    assert_eq!(stored_eth_addr, &ethereum_address);
}

#[test]
fn test_authority_transfer() {
    let (mut svm, program_id, authority) = setup();
    let user_account = Keypair::new();

    // Initialize user account
    let init_ix = Instruction::new_with_bytes(
        program_id,
        &external_delegate_token_master::instruction::Initialize {}.data(),
        external_delegate_token_master::accounts::Initialize {
            user_account: user_account.pubkey(),
            authority: authority.pubkey(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, &[init_ix], &authority, &[&user_account]);

    // user_pda is derived from user_account key
    let (user_pda, _bump) =
        Pubkey::find_program_address(&[user_account.pubkey().as_ref()], &program_id);

    // Create mint and token accounts
    svm.expire_blockhash();
    let mint = create_mint(&mut svm, &authority, 6);

    // Create ATA for the user_pda (the PDA holds the tokens)
    svm.expire_blockhash();
    let user_pda_ata = derive_ata(&user_pda, &mint.pubkey());
    let create_pda_ata = create_ata_ix(&authority.pubkey(), &user_pda, &mint.pubkey());
    send_tx(&mut svm, &[create_pda_ata], &authority, &[]);

    // Mint tokens to user_pda's ATA
    svm.expire_blockhash();
    let mint_amount: u64 = 1_000_000_000;
    let mint_ix = mint_to_ix(
        &mint.pubkey(),
        &user_pda_ata,
        &authority.pubkey(),
        mint_amount,
    );
    send_tx(&mut svm, &[mint_ix], &authority, &[]);

    // Create recipient ATA
    svm.expire_blockhash();
    let recipient = Keypair::new();
    let recipient_ata = derive_ata(&recipient.pubkey(), &mint.pubkey());
    let create_recipient_ata =
        create_ata_ix(&authority.pubkey(), &recipient.pubkey(), &mint.pubkey());
    send_tx(&mut svm, &[create_recipient_ata], &authority, &[]);

    // Perform authority transfer
    svm.expire_blockhash();
    let transfer_amount: u64 = 500_000_000;
    let authority_transfer_ix = Instruction::new_with_bytes(
        program_id,
        &external_delegate_token_master::instruction::AuthorityTransfer {
            amount: transfer_amount,
        }
        .data(),
        external_delegate_token_master::accounts::AuthorityTransfer {
            user_account: user_account.pubkey(),
            authority: authority.pubkey(),
            user_token_account: user_pda_ata,
            recipient_token_account: recipient_ata,
            user_pda,
            token_program: token_program_id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, &[authority_transfer_ix], &authority, &[]);

    // Verify recipient received tokens
    let recipient_data = svm
        .get_account(&recipient_ata)
        .expect("Recipient ATA should exist");
    assert_eq!(read_token_amount(&recipient_data.data), transfer_amount);

    // Verify user_pda's balance decreased
    let user_pda_data = svm
        .get_account(&user_pda_ata)
        .expect("User PDA ATA should still exist");
    assert_eq!(
        read_token_amount(&user_pda_data.data),
        mint_amount - transfer_amount
    );
}
