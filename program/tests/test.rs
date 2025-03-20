mod fixtures;

use crate::fixtures::TEST_AUTHORITY;
use curve25519_dalek::Scalar;
use ephemeral_vrf::vrf::{compute_vrf, generate_vrf_keypair, verify_vrf};
use ephemeral_vrf_api::prelude::*;
use solana_curve25519::ristretto::PodRistrettoPoint;
use solana_curve25519::scalar::PodScalar;
use solana_program::hash::Hash;
use solana_program_test::{processor, BanksClient, ProgramTest};
use solana_sdk::account::Account;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};
use std::str::FromStr;
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

    // Submit add oracle transaction.
    let new_oracle_keypair = Keypair::from(payer.insecure_clone());
    let new_oracle = new_oracle_keypair.pubkey();
    let ix = add_oracle(authority_keypair.pubkey(), new_oracle, new_oracle);
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
    assert!(oracles.oracles.iter().any(|o| o.identity == new_oracle));

    // Submit init oracle queue transaction.
    let ix = initialize_oracle_queue(payer.pubkey(), new_oracle, 0);
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[&payer], blockhash);
    let res = banks.process_transaction(tx).await;
    assert!(res.is_ok());

    // Verify queue was initialized.
    let oracle_queue_address = oracle_queue_pda(new_oracle, 0).0;
    let oracle_queue_account = banks
        .get_account(oracle_queue_address)
        .await
        .unwrap()
        .unwrap();
    let oracle_queue =
        QueueAccount::try_from_bytes_with_discriminator(&oracle_queue_account.data).unwrap();
    assert_eq!(oracle_queue_account.owner, ephemeral_vrf_api::ID);
    assert_eq!(oracle_queue.items.len(), 0);

    // Submit request for randomness transaction.
    let seed_pk = Pubkey::new_unique();
    let ix = request_randomness(
        payer.pubkey(),
        oracle_queue_address,
        ephemeral_vrf_api::ID,
        [0; 8],
        None,
        Some(seed_pk.to_bytes()),
    );
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[&payer], blockhash);
    let res = banks.process_transaction(tx).await;
    assert!(res.is_ok());

    // Verify request was added to queue.
    let oracle_queue_address = oracle_queue_pda(new_oracle, 0).0;
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
    let (sk, pk) = generate_vrf_keypair();
    let vrf_input = oracle_queue
        .items
        .iter()
        .collect::<Vec<_>>()
        .first()
        .unwrap()
        .0;
    let (output, (commitment_base_compressed, commitment_hash_compressed, s)) =
        compute_vrf(sk, vrf_input);

    // Verify generated randomness is correct.
    let verified = verify_vrf(
        pk,
        vrf_input,
        output,
        (commitment_base_compressed, commitment_hash_compressed, s),
    );
    assert!(verified);

    // Submit provide randomness transaction.
    let ix = provide_randomness(
        new_oracle,
        oracle_queue_address,
        *vrf_input,
        [0; 32],
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
    assert!(!oracles.oracles.iter().any(|o| o.identity == new_oracle));
}
