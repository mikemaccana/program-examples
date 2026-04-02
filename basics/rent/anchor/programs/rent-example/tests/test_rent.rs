use {
    anchor_lang::{
        solana_program::{instruction::Instruction, system_program},
        InstructionData, ToAccountMetas,
    },
    borsh::BorshDeserialize,
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

fn send_tx(svm: &mut LiteSVM, instruction: Instruction, payer: &Keypair, extra_signers: &[&Keypair]) {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);
    let mut signers: Vec<&Keypair> = vec![payer];
    signers.extend_from_slice(extra_signers);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &signers).unwrap();
    svm.send_transaction(tx).unwrap();
}

/// Build borsh-serialized AddressData bytes (fields are private in the crate).
fn build_address_data_borsh(name: &str, address: &str) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&(name.len() as u32).to_le_bytes());
    data.extend_from_slice(name.as_bytes());
    data.extend_from_slice(&(address.len() as u32).to_le_bytes());
    data.extend_from_slice(address.as_bytes());
    data
}

/// Construct the full instruction data with discriminator + AddressData.
/// Deserialize the borsh bytes into AddressData via the crate's BorshDeserialize impl,
/// then use InstructionData to get the final bytes.
fn build_create_system_account_ix_data(name: &str, address: &str) -> Vec<u8> {
    let address_data_bytes = build_address_data_borsh(name, address);
    let address_data =
        rent_example::AddressData::deserialize(&mut address_data_bytes.as_slice()).unwrap();
    rent_example::instruction::CreateSystemAccount { address_data }.data()
}

#[test]
fn test_create_system_account() {
    let program_id = rent_example::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/rent_example.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let new_account = Keypair::new();

    let name = "Marcus";
    let address = "123 Main St. San Francisco, CA";

    let ix_data = build_create_system_account_ix_data(name, address);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_data,
        rent_example::accounts::CreateSystemAccount {
            payer: payer.pubkey(),
            new_account: new_account.pubkey(),
            system_program: system_program::id(),
        }
        .to_account_metas(None),
    );

    send_tx(&mut svm, instruction, &payer, &[&new_account]);

    // Verify the account was created with the correct size
    // Borsh serialized AddressData: 4 + 6 ("Marcus") + 4 + 30 = 44 bytes
    let expected_size = 4 + name.len() + 4 + address.len();
    let account = svm.get_account(&new_account.pubkey()).unwrap();
    assert_eq!(account.data.len(), expected_size);
    assert!(account.lamports > 0, "Account should have lamports for rent");
}
