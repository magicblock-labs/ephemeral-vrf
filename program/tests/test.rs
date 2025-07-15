mod fixtures;

use crate::fixtures::{TEST_AUTHORITY, TEST_CALLBACK_PROGRAM, TEST_ORACLE};
use ephemeral_rollups_sdk::consts::DELEGATION_PROGRAM_ID;
use ephemeral_vrf::vrf::{compute_vrf, generate_vrf_keypair, verify_vrf};
use ephemeral_vrf_api::prelude::*;
use solana_curve25519::ristretto::PodRistrettoPoint;
use solana_curve25519::scalar::PodScalar;
use solana_program::hash::Hash;
use solana_program::rent::Rent;
use solana_program::sysvar::slot_hashes;
use solana_program_test::{processor, read_file, BanksClient, ProgramTest};
use solana_sdk::account::Account;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::{pubkey, signature::Keypair, signer::Signer, transaction::Transaction};
use steel::*;

async fn setup() -> (BanksClient, Keypair, Hash) {
    let mut program_test = ProgramTest::new(
        "ephemeral_vrf_program",
        ephemeral_vrf_api::ID,
        processor!(ephemeral_vrf_program::process_instruction),
    );

    // Setup the test authority
    program_test.add_account(
        Keypair::from_bytes(&TEST_AUTHORITY).unwrap().pubkey(),
        Account {
            lamports: 1_000_000_000,
            data: vec![],
            owner: system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Setup the oracle
    program_test.add_account(
        Keypair::from_bytes(&TEST_ORACLE).unwrap().pubkey(),
        Account {
            lamports: 1_000_000_000,
            data: vec![],
            owner: system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Setup program to test vrf-macro
    let data = read_file("tests/integration/use-randomness/target/deploy/use_randomness.so");
    program_test.add_account(
        pubkey!("CDiutifqugEkabdqwc5TK3FmSAgFpkP3RPE1642BCEhi"),
        Account {
            lamports: Rent::default().minimum_balance(data.len()).max(1),
            data,
            owner: solana_sdk::bpf_loader::id(),
            executable: true,
            rent_epoch: 0,
        },
    );

    // Setup delegation program
    let data = read_file("tests/integration/use-randomness/tests/fixtures/dlp.so");
    program_test.add_account(
        DELEGATION_PROGRAM_ID,
        Account {
            lamports: Rent::default().minimum_balance(data.len()).max(1),
            data,
            owner: solana_sdk::bpf_loader::id(),
            executable: true,
            rent_epoch: 0,
        },
    );

    program_test.prefer_bpf(true);
    program_test.start().await
}

#[tokio::test]
async fn run_test() {
    // Setup test
    let (banks, payer, blockhash) = setup().await;

    let authority_keypair = Keypair::from_bytes(&TEST_AUTHORITY).unwrap();
    let new_oracle_keypair = Keypair::from_bytes(&TEST_ORACLE).unwrap();

    // Submit initialize transaction.
    let ix = initialize(payer.pubkey());
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[&payer], blockhash);
    let res = banks.process_transaction(tx).await;
    assert!(res.is_ok());

    // Verify oracles was initialized.
    let oracles_address = oracles_pda().0;
    let oracles_account = banks.get_account(oracles_address).await.unwrap().unwrap();
    let oracles = Oracles::try_from_bytes_with_discriminator(&oracles_account.data).unwrap();
    assert_eq!(oracles_account.owner, ephemeral_vrf_api::ID);
    assert_eq!(oracles.oracles.len(), 0);

    println!("oracles_address: {:?}", oracles_address);
    println!("Oracles data: {:?}", oracles_account.data);

    // Submit add oracle transaction.
    let new_oracle = new_oracle_keypair.pubkey();
    let (oracle_vrf_sk, oracle_vrf_pk) = generate_vrf_keypair(&new_oracle_keypair);
    let ix = add_oracle(
        authority_keypair.pubkey(),
        new_oracle,
        oracle_vrf_pk.compress().to_bytes(),
    );

    println!(
        "oracle vrf pk: {:?}",
        Pubkey::from(oracle_vrf_pk.compress().to_bytes())
    );
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&authority_keypair.pubkey()),
        &[&authority_keypair],
        blockhash,
    );
    let res = banks.process_transaction(tx).await;
    assert!(res.is_ok());

    // Verify oracle was added.
    let oracles_info = banks.get_account(oracles_address).await.unwrap().unwrap();
    let oracles_data = oracles_info.data;
    let oracles = Oracles::try_from_bytes_with_discriminator(&oracles_data).unwrap();
    assert!(oracles.oracles.iter().any(|o| o.eq(&new_oracle)));

    let oracle_data_info = banks
        .get_account(oracle_data_pda(&new_oracle).0)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(oracle_data_info.owner, ephemeral_vrf_api::ID);
    let oracle_data = Oracle::try_from_bytes(&oracle_data_info.data).unwrap();
    assert!(oracle_data.registration_slot > 0);
    assert_eq!(
        oracle_data.vrf_pubkey.0,
        oracle_vrf_pk.compress().to_bytes()
    );

    // Submit init oracle queue transaction.
    let ix = initialize_oracle_queue(payer.pubkey(), new_oracle, 0);
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[&payer], blockhash);
    let res = banks.process_transaction(tx).await;
    assert!(res.is_ok());

    // Verify queue was initialized.
    let oracle_queue_address = oracle_queue_pda(&new_oracle, 0).0;
    let oracle_queue_account = banks
        .get_account(oracle_queue_address)
        .await
        .unwrap()
        .unwrap();
    let oracle_queue =
        QueueAccount::try_from_bytes_with_discriminator(&oracle_queue_account.data).unwrap();
    assert_eq!(oracle_queue_account.owner, ephemeral_vrf_api::ID);
    assert_eq!(oracle_queue.len(), 0);

    println!("oracle_data_address: {:?}", oracle_data_pda(&new_oracle).0);
    println!("Oracle data: {:?}", oracle_data_info.data);
    println!("oracle_queue_address: {:?}", oracle_queue_address);
    println!("Oracle queue data: {:?}", oracle_queue_account.data);

    // Submit request for randomness transaction.
    let ix = request_randomness(payer.pubkey(), 0);
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[&payer], blockhash);
    let res = banks.process_transaction(tx).await;
    assert!(res.is_ok());

    // Verify request was added to queue.
    let oracle_queue_address = oracle_queue_pda(&new_oracle, 0).0;
    let oracle_queue_account = banks
        .get_account(oracle_queue_address)
        .await
        .unwrap()
        .unwrap();
    let oracle_queue =
        QueueAccount::try_from_bytes_with_discriminator(&oracle_queue_account.data).unwrap();
    assert_eq!(oracle_queue_account.owner, ephemeral_vrf_api::ID);
    assert_eq!(oracle_queue.len(), 1);

    // Compute off-chain VRF
    let vrf_input = oracle_queue.items[0].clone().unwrap().id;
    let (output, (commitment_base_compressed, commitment_hash_compressed, s)) =
        compute_vrf(oracle_vrf_sk, &vrf_input);

    // Verify generated randomness is correct.
    let verified = verify_vrf(
        oracle_vrf_pk,
        &vrf_input,
        output,
        (commitment_base_compressed, commitment_hash_compressed, s),
    );
    assert!(verified);

    // Submit provide randomness transaction.
    let ix = provide_randomness(
        new_oracle,
        oracle_queue_address,
        TEST_CALLBACK_PROGRAM,
        vrf_input,
        PodRistrettoPoint(output.to_bytes()),
        PodRistrettoPoint(commitment_base_compressed.to_bytes()),
        PodRistrettoPoint(commitment_hash_compressed.to_bytes()),
        PodScalar(s.to_bytes()),
    );
    let compute_ix = ComputeBudgetInstruction::set_compute_unit_limit(2_000_000);
    let tx = Transaction::new_signed_with_payer(
        &[compute_ix, ix],
        Some(&new_oracle),
        &[&new_oracle_keypair],
        blockhash,
    );
    let res = banks.process_transaction(tx).await;
    assert!(res.is_ok());
    let oracle_queue_account = banks
        .get_account(oracle_queue_address)
        .await
        .unwrap()
        .unwrap();
    let oracle_queue =
        QueueAccount::try_from_bytes_with_discriminator(&oracle_queue_account.data).unwrap();
    assert_eq!(oracle_queue_account.owner, ephemeral_vrf_api::ID);
    assert_eq!(oracle_queue.len(), 0);
    assert_eq!(
        oracle_queue_account.lamports,
        banks
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(oracle_queue_account.data.len())
    );

    // Delegate oracle queue to new vrf-macro
    let ix = delegate_oracle_queue(new_oracle_keypair.pubkey(), oracle_queue_address, 0);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&new_oracle_keypair.pubkey()),
        &[&new_oracle_keypair],
        blockhash,
    );
    let res = banks.process_transaction(tx).await;
    assert!(res.is_ok());

    // Verify delegation was successful by checking the queue account owner
    let oracle_queue_account = banks
        .get_account(oracle_queue_address)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(oracle_queue_account.owner, DELEGATION_PROGRAM_ID);

    // Initialize a new oracle queue
    let ix = initialize_oracle_queue(payer.pubkey(), new_oracle, 1);
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[&payer], blockhash);
    let res = banks.process_transaction(tx).await;
    assert!(res.is_ok());

    // Close oracle queue.
    let ix = close_oracle_queue(new_oracle_keypair.pubkey(), 1);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&new_oracle_keypair.pubkey()),
        &[&new_oracle_keypair],
        blockhash,
    );
    let res = banks.process_transaction(tx).await;
    assert!(res.is_ok());

    // Verify oracle queue was closed
    let oracle_queue_account = banks
        .get_account(oracle_queue_pda(&new_oracle, 1).0)
        .await
        .unwrap();
    assert!(oracle_queue_account.is_none());

    // Submit remove oracle transaction.
    let new_oracle = Pubkey::new_unique();
    let ix = remove_oracle(authority_keypair.pubkey(), new_oracle);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&authority_keypair.pubkey()),
        &[&authority_keypair],
        blockhash,
    );
    let res = banks.process_transaction(tx).await;
    assert!(res.is_ok());

    // Verify oracle was removed.
    let oracles_info = banks.get_account(oracles_address).await.unwrap().unwrap();
    let oracles_data = oracles_info.data;
    let oracles = Oracles::try_from_bytes_with_discriminator(&oracles_data).unwrap();
    assert!(!oracles.oracles.iter().any(|o| o.eq(&new_oracle)));
    assert_eq!(
        oracles_info.lamports,
        banks
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(oracles_data.len())
    );
}

pub fn request_randomness(signer: Pubkey, client_seed: u8) -> Instruction {
    // Constants from the integration test instruction layout (IDL)
    const DISCRIMINATOR: [u8; 8] = [213, 5, 173, 166, 37, 236, 31, 18];

    // Default addresses as per instruction
    let oracle_queue = pubkey!("GKE6d7iv8kCBrsxr78W3xVdjGLLLJnxsGiuzrsZCGEvb");

    // Program identity PDA (seeded with "identity")
    let (program_identity, _) = Pubkey::find_program_address(&[IDENTITY], &TEST_CALLBACK_PROGRAM);

    println!("program_identity: {}", program_identity);

    // Construct account metas
    let accounts = vec![
        AccountMeta::new(signer, true),
        AccountMeta::new_readonly(program_identity, false),
        AccountMeta::new(oracle_queue, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(slot_hashes::ID, false),
        AccountMeta::new_readonly(ephemeral_vrf_api::ID, false),
    ];

    // Instruction data: discriminator + client_seed
    let mut data = DISCRIMINATOR.to_vec();
    data.push(client_seed);

    Instruction {
        program_id: TEST_CALLBACK_PROGRAM,
        accounts,
        data,
    }
}
