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

const TOKEN_PROGRAM_ID_STR: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const ATA_PROGRAM_ID_STR: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
const RENT_SYSVAR_STR: &str = "SysvarRent111111111111111111111111111111111";

fn token_program_id() -> Pubkey {
    TOKEN_PROGRAM_ID_STR.parse().unwrap()
}
fn ata_program_id() -> Pubkey {
    ATA_PROGRAM_ID_STR.parse().unwrap()
}
fn rent_sysvar_id() -> Pubkey {
    RENT_SYSVAR_STR.parse().unwrap()
}

/// Derive ATA address manually (same as spl_associated_token_account::get_associated_token_address).
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
    // Mint account is 82 bytes
    let rent_lamports = 1_461_600; // Rent-exempt for 82 bytes
    let create_account_ix = anchor_lang::solana_program::system_instruction::create_account(
        payer,
        mint,
        rent_lamports,
        82,
        &token_program_id(),
    );

    // InitializeMint2 instruction (no rent sysvar needed)
    // Instruction data: [20] + decimals(1) + mint_authority(32) + option(1) + freeze_authority(32)
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
    let program_id = swap_example::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!("../../../target/deploy/swap_example.so");
    svm.add_program(program_id, program_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 100_000_000_000).unwrap();
    (svm, program_id, payer)
}

/// Ensure mint_a < mint_b by pubkey ordering (the program may require this).
fn ordered_mints() -> (Keypair, Keypair) {
    loop {
        let a = Keypair::new();
        let b = Keypair::new();
        if a.pubkey().as_ref() < b.pubkey().as_ref() {
            return (a, b);
        }
    }
}

struct TestSetup {
    svm: LiteSVM,
    program_id: Pubkey,
    payer: Keypair,
    admin: Keypair,
    amm_id: Pubkey,
    amm_key: Pubkey,
    mint_a: Keypair,
    mint_b: Keypair,
    pool_key: Pubkey,
    pool_authority: Pubkey,
    mint_liquidity: Pubkey,
    pool_account_a: Pubkey,
    pool_account_b: Pubkey,
    holder_account_a: Pubkey,
    holder_account_b: Pubkey,
    liquidity_account: Pubkey,
}

fn full_setup() -> TestSetup {
    let (mut svm, program_id, payer) = setup();
    let admin = Keypair::new();
    svm.airdrop(&admin.pubkey(), 100_000_000_000).unwrap();

    let (mint_a, mint_b) = ordered_mints();
    let amm_id = Keypair::new().pubkey();
    let fee: u16 = 500;

    // Derive PDAs
    let (amm_key, _) = Pubkey::find_program_address(&[amm_id.as_ref()], &program_id);
    let (pool_key, _) = Pubkey::find_program_address(
        &[
            amm_key.as_ref(),
            mint_a.pubkey().as_ref(),
            mint_b.pubkey().as_ref(),
        ],
        &program_id,
    );
    let (pool_authority, _) = Pubkey::find_program_address(
        &[
            amm_key.as_ref(),
            mint_a.pubkey().as_ref(),
            mint_b.pubkey().as_ref(),
            b"authority",
        ],
        &program_id,
    );
    let (mint_liquidity, _) = Pubkey::find_program_address(
        &[
            amm_key.as_ref(),
            mint_a.pubkey().as_ref(),
            mint_b.pubkey().as_ref(),
            b"liquidity",
        ],
        &program_id,
    );

    let pool_account_a = derive_ata(&pool_authority, &mint_a.pubkey());
    let pool_account_b = derive_ata(&pool_authority, &mint_b.pubkey());
    let holder_account_a = derive_ata(&admin.pubkey(), &mint_a.pubkey());
    let holder_account_b = derive_ata(&admin.pubkey(), &mint_b.pubkey());
    let liquidity_account = derive_ata(&admin.pubkey(), &mint_liquidity);

    let decimals: u8 = 6;
    let minted_amount: u64 = 100 * 10u64.pow(decimals as u32);

    // 1. Create mints
    let mint_a_ixs =
        create_mint_instructions(&payer.pubkey(), &mint_a.pubkey(), &admin.pubkey(), decimals);
    send_tx(&mut svm, &mint_a_ixs, &payer, &[&mint_a]);

    svm.expire_blockhash();
    let mint_b_ixs =
        create_mint_instructions(&payer.pubkey(), &mint_b.pubkey(), &admin.pubkey(), decimals);
    send_tx(&mut svm, &mint_b_ixs, &payer, &[&mint_b]);

    // 2. Create ATAs for admin and mint tokens
    svm.expire_blockhash();
    let create_ata_a = create_ata_instruction(&payer.pubkey(), &admin.pubkey(), &mint_a.pubkey());
    let create_ata_b = create_ata_instruction(&payer.pubkey(), &admin.pubkey(), &mint_b.pubkey());
    send_tx(&mut svm, &[create_ata_a, create_ata_b], &payer, &[]);

    svm.expire_blockhash();
    let mint_to_a = mint_to_instruction(
        &mint_a.pubkey(),
        &holder_account_a,
        &admin.pubkey(),
        minted_amount,
    );
    let mint_to_b = mint_to_instruction(
        &mint_b.pubkey(),
        &holder_account_b,
        &admin.pubkey(),
        minted_amount,
    );
    send_tx(&mut svm, &[mint_to_a, mint_to_b], &payer, &[&admin]);

    // 3. Create AMM
    svm.expire_blockhash();
    let create_amm_ix = Instruction::new_with_bytes(
        program_id,
        &swap_example::instruction::CreateAmm {
            id: amm_id,
            fee,
        }
        .data(),
        swap_example::accounts::CreateAmm {
            amm: amm_key,
            admin: admin.pubkey(),
            payer: payer.pubkey(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, &[create_amm_ix], &payer, &[]);

    // 4. Create Pool
    svm.expire_blockhash();
    let create_pool_ix = Instruction::new_with_bytes(
        program_id,
        &swap_example::instruction::CreatePool {}.data(),
        swap_example::accounts::CreatePool {
            amm: amm_key,
            pool: pool_key,
            pool_authority,
            mint_liquidity,
            mint_a: mint_a.pubkey(),
            mint_b: mint_b.pubkey(),
            pool_account_a,
            pool_account_b,
            payer: payer.pubkey(),
            token_program: token_program_id(),
            associated_token_program: ata_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut svm, &[create_pool_ix], &payer, &[]);

    TestSetup {
        svm,
        program_id,
        payer,
        admin,
        amm_id,
        amm_key,
        mint_a,
        mint_b,
        pool_key,
        pool_authority,
        mint_liquidity,
        pool_account_a,
        pool_account_b,
        holder_account_a,
        holder_account_b,
        liquidity_account,
    }
}

#[test]
fn test_create_amm() {
    let (mut svm, program_id, payer) = setup();
    let amm_id = Keypair::new().pubkey();
    let fee: u16 = 500;
    let admin = Keypair::new();

    let (amm_key, _) = Pubkey::find_program_address(&[amm_id.as_ref()], &program_id);

    let create_amm_ix = Instruction::new_with_bytes(
        program_id,
        &swap_example::instruction::CreateAmm {
            id: amm_id,
            fee,
        }
        .data(),
        swap_example::accounts::CreateAmm {
            amm: amm_key,
            admin: admin.pubkey(),
            payer: payer.pubkey(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );

    send_tx(&mut svm, &[create_amm_ix], &payer, &[]);

    // Verify AMM account exists
    let amm_account = svm.get_account(&amm_key).expect("AMM account should exist");
    assert!(!amm_account.data.is_empty());
}

#[test]
fn test_deposit_liquidity() {
    let mut ts = full_setup();
    let deposit_amount_a: u64 = 4_000_000; // 4 tokens
    let deposit_amount_b: u64 = 1_000_000; // 1 token

    ts.svm.expire_blockhash();
    let deposit_ix = Instruction::new_with_bytes(
        ts.program_id,
        &swap_example::instruction::DepositLiquidity {
            amount_a: deposit_amount_a,
            amount_b: deposit_amount_b,
        }
        .data(),
        swap_example::accounts::DepositLiquidity {
            pool: ts.pool_key,
            pool_authority: ts.pool_authority,
            depositor: ts.admin.pubkey(),
            mint_liquidity: ts.mint_liquidity,
            mint_a: ts.mint_a.pubkey(),
            mint_b: ts.mint_b.pubkey(),
            pool_account_a: ts.pool_account_a,
            pool_account_b: ts.pool_account_b,
            depositor_account_liquidity: ts.liquidity_account,
            depositor_account_a: ts.holder_account_a,
            depositor_account_b: ts.holder_account_b,
            payer: ts.payer.pubkey(),
            token_program: token_program_id(),
            associated_token_program: ata_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );

    send_tx(&mut ts.svm, &[deposit_ix], &ts.payer, &[&ts.admin]);

    // Verify liquidity tokens were minted (should be sqrt(4M * 1M) - 100 = 2000 - 100 = 1900)
    let liq_data = ts
        .svm
        .get_account(&ts.liquidity_account)
        .expect("Liquidity account should exist");
    let liq_amount = read_token_amount(&liq_data.data);
    assert!(liq_amount > 0, "Should have received liquidity tokens");
}

#[test]
fn test_swap_a_to_b() {
    let mut ts = full_setup();

    // Deposit liquidity first
    let deposit_amount_a: u64 = 4_000_000;
    let deposit_amount_b: u64 = 1_000_000;

    ts.svm.expire_blockhash();
    let deposit_ix = Instruction::new_with_bytes(
        ts.program_id,
        &swap_example::instruction::DepositLiquidity {
            amount_a: deposit_amount_a,
            amount_b: deposit_amount_b,
        }
        .data(),
        swap_example::accounts::DepositLiquidity {
            pool: ts.pool_key,
            pool_authority: ts.pool_authority,
            depositor: ts.admin.pubkey(),
            mint_liquidity: ts.mint_liquidity,
            mint_a: ts.mint_a.pubkey(),
            mint_b: ts.mint_b.pubkey(),
            pool_account_a: ts.pool_account_a,
            pool_account_b: ts.pool_account_b,
            depositor_account_liquidity: ts.liquidity_account,
            depositor_account_a: ts.holder_account_a,
            depositor_account_b: ts.holder_account_b,
            payer: ts.payer.pubkey(),
            token_program: token_program_id(),
            associated_token_program: ata_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut ts.svm, &[deposit_ix], &ts.payer, &[&ts.admin]);

    // Get balances before swap
    let before_b = read_token_amount(
        &ts.svm
            .get_account(&ts.holder_account_b)
            .unwrap()
            .data,
    );

    // Swap 1M of token A for token B
    ts.svm.expire_blockhash();
    let input_amount: u64 = 1_000_000;
    let swap_ix = Instruction::new_with_bytes(
        ts.program_id,
        &swap_example::instruction::SwapExactTokensForTokens {
            swap_a: true,
            input_amount,
            min_output_amount: 100,
        }
        .data(),
        swap_example::accounts::SwapExactTokensForTokens {
            amm: ts.amm_key,
            pool: ts.pool_key,
            pool_authority: ts.pool_authority,
            trader: ts.admin.pubkey(),
            mint_a: ts.mint_a.pubkey(),
            mint_b: ts.mint_b.pubkey(),
            pool_account_a: ts.pool_account_a,
            pool_account_b: ts.pool_account_b,
            trader_account_a: ts.holder_account_a,
            trader_account_b: ts.holder_account_b,
            payer: ts.payer.pubkey(),
            token_program: token_program_id(),
            associated_token_program: ata_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut ts.svm, &[swap_ix], &ts.payer, &[&ts.admin]);

    // After swap, token B balance should have increased
    let after_b = read_token_amount(
        &ts.svm
            .get_account(&ts.holder_account_b)
            .unwrap()
            .data,
    );
    assert!(
        after_b > before_b,
        "Token B balance should increase after swap A->B"
    );
}

#[test]
fn test_withdraw_liquidity() {
    let mut ts = full_setup();

    // Deposit liquidity
    let deposit_amount: u64 = 4_000_000;

    ts.svm.expire_blockhash();
    let deposit_ix = Instruction::new_with_bytes(
        ts.program_id,
        &swap_example::instruction::DepositLiquidity {
            amount_a: deposit_amount,
            amount_b: deposit_amount,
        }
        .data(),
        swap_example::accounts::DepositLiquidity {
            pool: ts.pool_key,
            pool_authority: ts.pool_authority,
            depositor: ts.admin.pubkey(),
            mint_liquidity: ts.mint_liquidity,
            mint_a: ts.mint_a.pubkey(),
            mint_b: ts.mint_b.pubkey(),
            pool_account_a: ts.pool_account_a,
            pool_account_b: ts.pool_account_b,
            depositor_account_liquidity: ts.liquidity_account,
            depositor_account_a: ts.holder_account_a,
            depositor_account_b: ts.holder_account_b,
            payer: ts.payer.pubkey(),
            token_program: token_program_id(),
            associated_token_program: ata_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut ts.svm, &[deposit_ix], &ts.payer, &[&ts.admin]);

    // Get liquidity token balance
    let liq_data = ts.svm.get_account(&ts.liquidity_account).unwrap();
    let liq_amount = read_token_amount(&liq_data.data);
    assert!(liq_amount > 0);

    // Withdraw all liquidity
    ts.svm.expire_blockhash();
    let withdraw_ix = Instruction::new_with_bytes(
        ts.program_id,
        &swap_example::instruction::WithdrawLiquidity {
            amount: liq_amount,
        }
        .data(),
        swap_example::accounts::WithdrawLiquidity {
            amm: ts.amm_key,
            pool: ts.pool_key,
            pool_authority: ts.pool_authority,
            depositor: ts.admin.pubkey(),
            mint_liquidity: ts.mint_liquidity,
            mint_a: ts.mint_a.pubkey(),
            mint_b: ts.mint_b.pubkey(),
            pool_account_a: ts.pool_account_a,
            pool_account_b: ts.pool_account_b,
            depositor_account_liquidity: ts.liquidity_account,
            depositor_account_a: ts.holder_account_a,
            depositor_account_b: ts.holder_account_b,
            payer: ts.payer.pubkey(),
            token_program: token_program_id(),
            associated_token_program: ata_program_id(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );
    send_tx(&mut ts.svm, &[withdraw_ix], &ts.payer, &[&ts.admin]);

    // Liquidity balance should be 0
    let liq_data = ts.svm.get_account(&ts.liquidity_account).unwrap();
    let liq_amount = read_token_amount(&liq_data.data);
    assert_eq!(liq_amount, 0, "Liquidity should be fully withdrawn");
}
