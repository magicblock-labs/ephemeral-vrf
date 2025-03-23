mod fixtures;

use crate::fixtures::{TEST_AUTHORITY, TEST_CALLBACK_PROGRAM, TEST_CALLBACK_DISCRIMINATOR};
use ephemeral_vrf::vrf::{compute_vrf, generate_vrf_keypair, verify_vrf};
use ephemeral_vrf_api::prelude::*;
use solana_curve25519::ristretto::PodRistrettoPoint;
use solana_curve25519::scalar::PodScalar;
use solana_program::hash::Hash;
use solana_program::rent::Rent;
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

    // Setup program to test callback
    let data = read_file("tests/integration/use-randomness/target/deploy/use_randomness.so");
    program_test.add_account(
        pubkey!("AL32mNVFdhxHXztaWuNWvwoiPYCHofWmVRNH49pMCafD"),
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
    let new_oracle_keypair = Keypair::from(payer.insecure_clone());
    let new_oracle = new_oracle_keypair.pubkey();
    let (oracle_vrf_sk, oracle_vrf_pk) = generate_vrf_keypair(&payer);
    let ix = add_oracle(
        authority_keypair.pubkey(),
        new_oracle,
        oracle_vrf_pk.compress().to_bytes(),
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
    assert_eq!(oracle_queue.items.len(), 0);

    println!("oracle_data_address: {:?}", oracle_data_pda(&new_oracle).0);
    println!("Oracle data: {:?}", oracle_data_info.data);
    println!("oracle_queue_address: {:?}", oracle_queue_address);
    println!("Oracle queue data: {:?}", oracle_queue_account.data);

    // Submit request for randomness transaction.
    let seed_pk = Pubkey::new_unique();
    let ix = request_randomness(
        payer.pubkey(),
        oracle_queue_address,
        TEST_CALLBACK_PROGRAM,
        TEST_CALLBACK_DISCRIMINATOR.to_vec(),
        None,
        Some(seed_pk.to_bytes()),
    );
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
    assert_eq!(oracle_queue.items.len(), 1);
    assert_eq!(
        oracle_queue
            .items
            .iter()
            .map(|(_, v)| v)
            .collect::<Vec<_>>()
            .first()
            .unwrap()
            .seed,
        seed_pk.to_bytes()
    );

    // Compute off-chain VRF
    let vrf_input = oracle_queue
        .items
        .iter()
        .collect::<Vec<_>>()
        .first()
        .unwrap()
        .0;
    let (output, (commitment_base_compressed, commitment_hash_compressed, s)) =
        compute_vrf(oracle_vrf_sk, vrf_input);

    // Verify generated randomness is correct.
    let verified = verify_vrf(
        oracle_vrf_pk,
        vrf_input,
        output,
        (commitment_base_compressed, commitment_hash_compressed, s),
    );
    assert!(verified);

    // Submit provide randomness transaction.
    let ix = provide_randomness(
        new_oracle,
        oracle_queue_address,
        TEST_CALLBACK_PROGRAM,
        *vrf_input,
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
    assert_eq!(oracle_queue.items.len(), 0);
    assert_eq!(
        oracle_queue_account.lamports,
        banks
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(oracle_queue_account.data.len())
    );

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
