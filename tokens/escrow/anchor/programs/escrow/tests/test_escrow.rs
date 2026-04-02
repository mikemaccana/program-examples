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

/// Build a CreateAccount + InitializeMint2 instruction pair for creating an SPL mint.
fn create_mint_instructions(
    payer: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
    decimals: u8,
) -> Vec<Instruction> {
    let rent_lamports = 1_461_600; // Rent-exempt for 82 bytes (mint account)
    let create_account_ix = anchor_lang::solana_program::system_instruction::create_account(
        payer,
        mint,
        rent_lamports,
        82,
        &token_program_id(),
    );

    // InitializeMint2 instruction (no rent sysvar needed)
    let mut init_data = vec![20u8]; // InitializeMint2 = index 20
    init_data.push(decimals);
    init_data.extend_from_slice(authority.as_ref());
    init_data.push(1); // Some freeze authority
    init_data.extend_from_slice(authority.as_ref());

    let init_mint_ix = Instruction {
        program_id: token_program_id(),
        accounts: vec![anchor_lang::solana_program::instruction::AccountMeta::new(
            *mint, false,
        )],
        data: init_data,
    };

    vec![create_account_ix, init_mint_ix]
}

/// Build a CreateAssociatedTokenAccount instruction.
fn create_ata_instruction(payer: &Pubkey, wallet: &Pubkey, mint: &Pubkey) -> Instruction {
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
        data: vec![0], // Create instruction
    }
}

/// Build a MintTo instruction.
fn mint_to_instruction(
    mint: &Pubkey,
    destination: &Pubkey,
    authority: &Pubkey,
    amount: u64,
) -> Instruction {
    let mut data = vec![7u8]; // MintTo = index 7
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: token_program_id(),
        accounts: vec![
            anchor_lang::solana_program::instruction::AccountMeta::new(*mint, false),
            anchor_lang::solana_program::instruction::AccountMeta::new(*destination, false),
            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(*authority, true),
        ],
        data,
    }
}

fn setup() -> (LiteSVM, Pubkey, Keypair) {
    let program_id = escrow::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/escrow.so");
    svm.add_program(program_id, program_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 100_000_000_000).unwrap();
    (svm, program_id, payer)
}

/// Full escrow setup: creates two mints, funds Alice and Bob, creates ATAs.
struct EscrowSetup {
    svm: LiteSVM,
    program_id: Pubkey,
    payer: Keypair,
    alice: Keypair,
    bob: Keypair,
    mint_a: Keypair,
    mint_b: Keypair,
    alice_ata_a: Pubkey,
    alice_ata_b: Pubkey,
    bob_ata_a: Pubkey,
    bob_ata_b: Pubkey,
}

fn full_setup() -> EscrowSetup {
    let (mut svm, program_id, payer) = setup();

    let alice = Keypair::new();
    let bob = Keypair::new();
    let mint_a = Keypair::new();
    let mint_b = Keypair::new();

    svm.airdrop(&alice.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&bob.pubkey(), 10_000_000_000).unwrap();

    let decimals: u8 = 6;
    let alice_amount: u64 = 1_000_000_000;
    let bob_amount: u64 = 1_000_000_000;

    // Create mint A
    let mint_a_ixs = create_mint_instructions(
        &payer.pubkey(),
        &mint_a.pubkey(),
        &payer.pubkey(),
        decimals,
    );
    send_tx(&mut svm, &mint_a_ixs, &payer, &[&mint_a]);

    // Create mint B
    svm.expire_blockhash();
    let mint_b_ixs = create_mint_instructions(
        &payer.pubkey(),
        &mint_b.pubkey(),
        &payer.pubkey(),
        decimals,
    );
    send_tx(&mut svm, &mint_b_ixs, &payer, &[&mint_b]);

    // Create ATAs
    let alice_ata_a = derive_ata(&alice.pubkey(), &mint_a.pubkey());
    let alice_ata_b = derive_ata(&alice.pubkey(), &mint_b.pubkey());
    let bob_ata_a = derive_ata(&bob.pubkey(), &mint_a.pubkey());
    let bob_ata_b = derive_ata(&bob.pubkey(), &mint_b.pubkey());

    svm.expire_blockhash();
    let create_alice_ata_a = create_ata_instruction(&payer.pubkey(), &alice.pubkey(), &mint_a.pubkey());
    let create_alice_ata_b = create_ata_instruction(&payer.pubkey(), &alice.pubkey(), &mint_b.pubkey());
    let create_bob_ata_b = create_ata_instruction(&payer.pubkey(), &bob.pubkey(), &mint_b.pubkey());
    send_tx(
        &mut svm,
        &[create_alice_ata_a, create_alice_ata_b, create_bob_ata_b],
        &payer,
        &[],
    );

    // Mint tokens: Alice gets token A, Bob gets token B
    svm.expire_blockhash();
    let mint_to_alice_a = mint_to_instruction(
        &mint_a.pubkey(),
        &alice_ata_a,
        &payer.pubkey(),
        alice_amount,
    );
    let mint_to_bob_b = mint_to_instruction(
        &mint_b.pubkey(),
        &bob_ata_b,
        &payer.pubkey(),
        bob_amount,
    );
    send_tx(&mut svm, &[mint_to_alice_a, mint_to_bob_b], &payer, &[]);

    EscrowSetup {
        svm,
        program_id,
        payer,
        alice,
        bob,
        mint_a,
        mint_b,
        alice_ata_a,
        alice_ata_b,
        bob_ata_a,
        bob_ata_b,
    }
}

#[test]
fn test_make_offer() {
    let mut es = full_setup();

    let offer_id: u64 = 1;
    let token_a_offered_amount: u64 = 1_000_000;
    let token_b_wanted_amount: u64 = 1_000_000;

    // Derive offer PDA
    let (offer_pda, _bump) = Pubkey::find_program_address(
        &[
            b"offer",
            es.alice.pubkey().as_ref(),
            &offer_id.to_le_bytes(),
        ],
        &es.program_id,
    );

    // Vault is the ATA of the offer PDA for mint_a
    let vault = derive_ata(&offer_pda, &es.mint_a.pubkey());

    es.svm.expire_blockhash();
    let make_offer_ix = Instruction::new_with_bytes(
        es.program_id,
        &escrow::instruction::MakeOffer {
            id: offer_id,
            token_a_offered_amount,
            token_b_wanted_amount,
        }
        .data(),
        escrow::accounts::MakeOffer {
            maker: es.alice.pubkey(),
            token_mint_a: es.mint_a.pubkey(),
            token_mint_b: es.mint_b.pubkey(),
            maker_token_account_a: es.alice_ata_a,
            offer: offer_pda,
            vault,
            associated_token_program: ata_program_id(),
            token_program: token_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );

    send_tx(&mut es.svm, &[make_offer_ix], &es.payer, &[&es.alice]);

    // Verify vault contains the offered tokens
    let vault_data = es.svm.get_account(&vault).expect("Vault should exist");
    let vault_balance = read_token_amount(&vault_data.data);
    assert_eq!(vault_balance, token_a_offered_amount);

    // Verify offer account data
    let offer_data = es.svm.get_account(&offer_pda).expect("Offer should exist");
    // Skip 8-byte discriminator, then read the Offer struct fields
    let data = &offer_data.data[8..];
    // id: u64 (8 bytes)
    let stored_id = u64::from_le_bytes(data[0..8].try_into().unwrap());
    assert_eq!(stored_id, offer_id);
    // maker: Pubkey (32 bytes)
    let stored_maker = Pubkey::try_from(&data[8..40]).unwrap();
    assert_eq!(stored_maker, es.alice.pubkey());
}

#[test]
fn test_take_offer() {
    let mut es = full_setup();

    let offer_id: u64 = 2;
    let token_a_offered_amount: u64 = 1_000_000;
    let token_b_wanted_amount: u64 = 1_000_000;

    // Derive offer PDA
    let (offer_pda, _bump) = Pubkey::find_program_address(
        &[
            b"offer",
            es.alice.pubkey().as_ref(),
            &offer_id.to_le_bytes(),
        ],
        &es.program_id,
    );

    let vault = derive_ata(&offer_pda, &es.mint_a.pubkey());

    // Step 1: Alice makes the offer
    es.svm.expire_blockhash();
    let make_offer_ix = Instruction::new_with_bytes(
        es.program_id,
        &escrow::instruction::MakeOffer {
            id: offer_id,
            token_a_offered_amount,
            token_b_wanted_amount,
        }
        .data(),
        escrow::accounts::MakeOffer {
            maker: es.alice.pubkey(),
            token_mint_a: es.mint_a.pubkey(),
            token_mint_b: es.mint_b.pubkey(),
            maker_token_account_a: es.alice_ata_a,
            offer: offer_pda,
            vault,
            associated_token_program: ata_program_id(),
            token_program: token_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );

    send_tx(&mut es.svm, &[make_offer_ix], &es.payer, &[&es.alice]);

    // Verify vault has tokens
    let vault_data = es.svm.get_account(&vault).unwrap();
    assert_eq!(read_token_amount(&vault_data.data), token_a_offered_amount);

    // Step 2: Bob takes the offer
    // Bob needs ATA for token A (taker_token_account_a) — the program uses init_if_needed
    // Alice needs ATA for token B (maker_token_account_b) — the program uses init_if_needed
    es.svm.expire_blockhash();
    let take_offer_ix = Instruction::new_with_bytes(
        es.program_id,
        &escrow::instruction::TakeOffer {}.data(),
        escrow::accounts::TakeOffer {
            taker: es.bob.pubkey(),
            maker: es.alice.pubkey(),
            token_mint_a: es.mint_a.pubkey(),
            token_mint_b: es.mint_b.pubkey(),
            taker_token_account_a: es.bob_ata_a,
            taker_token_account_b: es.bob_ata_b,
            maker_token_account_b: es.alice_ata_b,
            offer: offer_pda,
            vault,
            associated_token_program: ata_program_id(),
            token_program: token_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );

    send_tx(&mut es.svm, &[take_offer_ix], &es.payer, &[&es.bob]);

    // Verify Bob received token A from vault
    let bob_ata_a_data = es.svm.get_account(&es.bob_ata_a).expect("Bob ATA A should exist");
    let bob_token_a_balance = read_token_amount(&bob_ata_a_data.data);
    assert_eq!(bob_token_a_balance, token_a_offered_amount);

    // Verify Alice received token B from Bob
    let alice_ata_b_data = es.svm.get_account(&es.alice_ata_b).expect("Alice ATA B should exist");
    let alice_token_b_balance = read_token_amount(&alice_ata_b_data.data);
    assert_eq!(alice_token_b_balance, token_b_wanted_amount);

    // Verify vault is closed (no longer exists)
    assert!(
        es.svm.get_account(&vault).is_none(),
        "Vault should be closed after take_offer"
    );

    // Verify offer account is closed
    assert!(
        es.svm.get_account(&offer_pda).is_none(),
        "Offer should be closed after take_offer"
    );
}
