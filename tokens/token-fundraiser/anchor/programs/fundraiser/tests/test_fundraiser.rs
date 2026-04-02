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
    let program_id = fundraiser::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/fundraiser.so");
    svm.add_program(program_id, program_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 100_000_000_000).unwrap();
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

struct FundraiserSetup {
    svm: LiteSVM,
    program_id: Pubkey,
    payer: Keypair,
    maker: Keypair,
    mint: Keypair,
    fundraiser_pda: Pubkey,
    vault: Pubkey,
}

fn full_setup() -> FundraiserSetup {
    let (mut svm, program_id, payer) = setup();

    let maker = Keypair::new();
    svm.airdrop(&maker.pubkey(), 10_000_000_000).unwrap();

    // Create mint (6 decimals)
    svm.expire_blockhash();
    let mint = create_mint(&mut svm, &payer, 6);

    // Derive the fundraiser PDA
    let (fundraiser_pda, _bump) = Pubkey::find_program_address(
        &[b"fundraiser", maker.pubkey().as_ref()],
        &program_id,
    );

    // Vault is the ATA of the fundraiser PDA for the mint
    let vault = derive_ata(&fundraiser_pda, &mint.pubkey());

    FundraiserSetup {
        svm,
        program_id,
        payer,
        maker,
        mint,
        fundraiser_pda,
        vault,
    }
}

#[test]
fn test_initialize_fundraiser() {
    let mut fs = full_setup();

    // amount_to_raise must be >= MIN_AMOUNT_TO_RAISE^decimals = 3^6 = 729
    let amount_to_raise: u64 = 30_000_000; // 30 tokens with 6 decimals
    let duration: u16 = 0; // Duration 0 = allows contributions immediately

    fs.svm.expire_blockhash();
    let init_ix = Instruction::new_with_bytes(
        fs.program_id,
        &fundraiser::instruction::Initialize {
            amount: amount_to_raise,
            duration,
        }
        .data(),
        fundraiser::accounts::Initialize {
            maker: fs.maker.pubkey(),
            mint_to_raise: fs.mint.pubkey(),
            fundraiser: fs.fundraiser_pda,
            vault: fs.vault,
            system_program: system_program::id(),
            token_program: token_program_id(),
            associated_token_program: ata_program_id(),
        }
        .to_account_metas(None),
    );

    send_tx(&mut fs.svm, &[init_ix], &fs.maker, &[]);

    // Verify fundraiser account exists
    let fundraiser_data = fs
        .svm
        .get_account(&fs.fundraiser_pda)
        .expect("Fundraiser account should exist");
    assert!(!fundraiser_data.data.is_empty());

    // Verify vault exists
    let vault_data = fs
        .svm
        .get_account(&fs.vault)
        .expect("Vault should exist");
    assert_eq!(read_token_amount(&vault_data.data), 0);
}

#[test]
fn test_contribute_and_refund() {
    let mut fs = full_setup();

    let amount_to_raise: u64 = 30_000_000;
    let duration: u16 = 0;

    // Initialize fundraiser
    fs.svm.expire_blockhash();
    let init_ix = Instruction::new_with_bytes(
        fs.program_id,
        &fundraiser::instruction::Initialize {
            amount: amount_to_raise,
            duration,
        }
        .data(),
        fundraiser::accounts::Initialize {
            maker: fs.maker.pubkey(),
            mint_to_raise: fs.mint.pubkey(),
            fundraiser: fs.fundraiser_pda,
            vault: fs.vault,
            system_program: system_program::id(),
            token_program: token_program_id(),
            associated_token_program: ata_program_id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut fs.svm, &[init_ix], &fs.maker, &[]);

    // Setup contributor: create ATA and mint tokens
    let contributor = Keypair::new();
    fs.svm.airdrop(&contributor.pubkey(), 10_000_000_000).unwrap();

    fs.svm.expire_blockhash();
    let contributor_ata = derive_ata(&contributor.pubkey(), &fs.mint.pubkey());
    let create_contributor_ata =
        create_ata_ix(&fs.payer.pubkey(), &contributor.pubkey(), &fs.mint.pubkey());
    send_tx(&mut fs.svm, &[create_contributor_ata], &fs.payer, &[]);

    fs.svm.expire_blockhash();
    let mint_amount: u64 = 10_000_000; // 10 tokens
    let mint_ix = mint_to_ix(
        &fs.mint.pubkey(),
        &contributor_ata,
        &fs.payer.pubkey(),
        mint_amount,
    );
    send_tx(&mut fs.svm, &[mint_ix], &fs.payer, &[]);

    // Derive contributor account PDA
    let (contributor_account_pda, _bump) = Pubkey::find_program_address(
        &[
            b"contributor",
            fs.fundraiser_pda.as_ref(),
            contributor.pubkey().as_ref(),
        ],
        &fs.program_id,
    );

    // Contribute 1_000_000 (max 10% of 30_000_000 = 3_000_000)
    fs.svm.expire_blockhash();
    let contribute_amount: u64 = 1_000_000;
    let contribute_ix = Instruction::new_with_bytes(
        fs.program_id,
        &fundraiser::instruction::Contribute {
            amount: contribute_amount,
        }
        .data(),
        fundraiser::accounts::Contribute {
            contributor: contributor.pubkey(),
            mint_to_raise: fs.mint.pubkey(),
            fundraiser: fs.fundraiser_pda,
            contributor_account: contributor_account_pda,
            contributor_ata,
            vault: fs.vault,
            token_program: token_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut fs.svm, &[contribute_ix], &contributor, &[]);

    // Verify vault balance
    let vault_data = fs.svm.get_account(&fs.vault).unwrap();
    assert_eq!(read_token_amount(&vault_data.data), contribute_amount);

    // Contribute again
    fs.svm.expire_blockhash();
    let contribute_ix2 = Instruction::new_with_bytes(
        fs.program_id,
        &fundraiser::instruction::Contribute {
            amount: contribute_amount,
        }
        .data(),
        fundraiser::accounts::Contribute {
            contributor: contributor.pubkey(),
            mint_to_raise: fs.mint.pubkey(),
            fundraiser: fs.fundraiser_pda,
            contributor_account: contributor_account_pda,
            contributor_ata,
            vault: fs.vault,
            token_program: token_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut fs.svm, &[contribute_ix2], &contributor, &[]);

    // Verify vault balance is now 2_000_000
    let vault_data = fs.svm.get_account(&fs.vault).unwrap();
    assert_eq!(
        read_token_amount(&vault_data.data),
        contribute_amount * 2
    );

    // Refund (target not met: 2_000_000 < 30_000_000, and duration=0 allows refund)
    fs.svm.expire_blockhash();
    let refund_ix = Instruction::new_with_bytes(
        fs.program_id,
        &fundraiser::instruction::Refund {}.data(),
        fundraiser::accounts::Refund {
            contributor: contributor.pubkey(),
            maker: fs.maker.pubkey(),
            mint_to_raise: fs.mint.pubkey(),
            fundraiser: fs.fundraiser_pda,
            contributor_account: contributor_account_pda,
            contributor_ata,
            vault: fs.vault,
            token_program: token_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut fs.svm, &[refund_ix], &contributor, &[]);

    // Verify vault is empty after refund
    let vault_data = fs.svm.get_account(&fs.vault).unwrap();
    assert_eq!(read_token_amount(&vault_data.data), 0);

    // Verify contributor got tokens back
    let contributor_data = fs.svm.get_account(&contributor_ata).unwrap();
    assert_eq!(read_token_amount(&contributor_data.data), mint_amount);

    // Contributor account PDA should be closed
    assert!(
        fs.svm.get_account(&contributor_account_pda).is_none(),
        "Contributor account should be closed after refund"
    );
}

#[test]
fn test_check_contributions_success() {
    let mut fs = full_setup();

    // Use a small amount_to_raise so we can reach the goal easily
    // MIN_AMOUNT_TO_RAISE^decimals = 3^6 = 729
    let amount_to_raise: u64 = 1_000; // Just above minimum
    let duration: u16 = 0;

    // Initialize fundraiser
    fs.svm.expire_blockhash();
    let init_ix = Instruction::new_with_bytes(
        fs.program_id,
        &fundraiser::instruction::Initialize {
            amount: amount_to_raise,
            duration,
        }
        .data(),
        fundraiser::accounts::Initialize {
            maker: fs.maker.pubkey(),
            mint_to_raise: fs.mint.pubkey(),
            fundraiser: fs.fundraiser_pda,
            vault: fs.vault,
            system_program: system_program::id(),
            token_program: token_program_id(),
            associated_token_program: ata_program_id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut fs.svm, &[init_ix], &fs.maker, &[]);

    // Setup contributor
    let contributor = Keypair::new();
    fs.svm.airdrop(&contributor.pubkey(), 10_000_000_000).unwrap();

    fs.svm.expire_blockhash();
    let contributor_ata = derive_ata(&contributor.pubkey(), &fs.mint.pubkey());
    let create_contributor_ata =
        create_ata_ix(&fs.payer.pubkey(), &contributor.pubkey(), &fs.mint.pubkey());
    send_tx(&mut fs.svm, &[create_contributor_ata], &fs.payer, &[]);

    fs.svm.expire_blockhash();
    let mint_ix = mint_to_ix(
        &fs.mint.pubkey(),
        &contributor_ata,
        &fs.payer.pubkey(),
        10_000,
    );
    send_tx(&mut fs.svm, &[mint_ix], &fs.payer, &[]);

    // Derive contributor account PDA
    let (contributor_account_pda, _bump) = Pubkey::find_program_address(
        &[
            b"contributor",
            fs.fundraiser_pda.as_ref(),
            contributor.pubkey().as_ref(),
        ],
        &fs.program_id,
    );

    // Contribute the max (10% of 1000 = 100)
    fs.svm.expire_blockhash();
    let contribute_ix = Instruction::new_with_bytes(
        fs.program_id,
        &fundraiser::instruction::Contribute {
            amount: 100,
        }
        .data(),
        fundraiser::accounts::Contribute {
            contributor: contributor.pubkey(),
            mint_to_raise: fs.mint.pubkey(),
            fundraiser: fs.fundraiser_pda,
            contributor_account: contributor_account_pda,
            contributor_ata,
            vault: fs.vault,
            token_program: token_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut fs.svm, &[contribute_ix], &contributor, &[]);

    // We need enough contributors to reach the goal
    // Each can contribute at most 10% = 100, and goal is 1000
    // So we need 10 contributors
    for i in 1..10 {
        let extra_contributor = Keypair::new();
        fs.svm
            .airdrop(&extra_contributor.pubkey(), 10_000_000_000)
            .unwrap();

        fs.svm.expire_blockhash();
        let extra_ata = derive_ata(&extra_contributor.pubkey(), &fs.mint.pubkey());
        let create_extra_ata = create_ata_ix(
            &fs.payer.pubkey(),
            &extra_contributor.pubkey(),
            &fs.mint.pubkey(),
        );
        send_tx(&mut fs.svm, &[create_extra_ata], &fs.payer, &[]);

        fs.svm.expire_blockhash();
        let mint_ix = mint_to_ix(
            &fs.mint.pubkey(),
            &extra_ata,
            &fs.payer.pubkey(),
            10_000,
        );
        send_tx(&mut fs.svm, &[mint_ix], &fs.payer, &[]);

        let (extra_contributor_pda, _) = Pubkey::find_program_address(
            &[
                b"contributor",
                fs.fundraiser_pda.as_ref(),
                extra_contributor.pubkey().as_ref(),
            ],
            &fs.program_id,
        );

        fs.svm.expire_blockhash();
        let extra_contribute_ix = Instruction::new_with_bytes(
            fs.program_id,
            &fundraiser::instruction::Contribute {
                amount: 100,
            }
            .data(),
            fundraiser::accounts::Contribute {
                contributor: extra_contributor.pubkey(),
                mint_to_raise: fs.mint.pubkey(),
                fundraiser: fs.fundraiser_pda,
                contributor_account: extra_contributor_pda,
                contributor_ata: extra_ata,
                vault: fs.vault,
                token_program: token_program_id(),
                system_program: system_program::id(),
            }
            .to_account_metas(None),
        );
        send_tx(&mut fs.svm, &[extra_contribute_ix], &extra_contributor, &[]);

        // Check if we've hit the goal after this contribution
        let vault_data = fs.svm.get_account(&fs.vault).unwrap();
        let current = read_token_amount(&vault_data.data);
        if current >= amount_to_raise {
            break;
        }
        let _ = i;
    }

    // Verify vault has enough
    let vault_data = fs.svm.get_account(&fs.vault).unwrap();
    assert!(read_token_amount(&vault_data.data) >= amount_to_raise);

    // Check contributions (maker claims the funds)
    let maker_ata = derive_ata(&fs.maker.pubkey(), &fs.mint.pubkey());

    fs.svm.expire_blockhash();
    let check_ix = Instruction::new_with_bytes(
        fs.program_id,
        &fundraiser::instruction::CheckContributions {}.data(),
        fundraiser::accounts::CheckContributions {
            maker: fs.maker.pubkey(),
            mint_to_raise: fs.mint.pubkey(),
            fundraiser: fs.fundraiser_pda,
            vault: fs.vault,
            maker_ata,
            token_program: token_program_id(),
            system_program: system_program::id(),
            associated_token_program: ata_program_id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut fs.svm, &[check_ix], &fs.maker, &[]);

    // Verify maker received the funds
    let maker_ata_data = fs
        .svm
        .get_account(&maker_ata)
        .expect("Maker ATA should exist");
    assert!(read_token_amount(&maker_ata_data.data) >= amount_to_raise);

    // Fundraiser account should be closed
    assert!(
        fs.svm.get_account(&fs.fundraiser_pda).is_none(),
        "Fundraiser account should be closed after check_contributions"
    );
}
