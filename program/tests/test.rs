use ephemeral_vrf_api::prelude::*;
use solana_program::hash::Hash;
use solana_program_test::{processor, BanksClient, ProgramTest};
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};
use steel::*;

async fn setup() -> (BanksClient, Keypair, Hash) {
    let mut program_test = ProgramTest::new(
        "ephemeral_vrf_program",
        ephemeral_vrf_api::ID,
        processor!(ephemeral_vrf_program::process_instruction),
    );
    program_test.prefer_bpf(true);
    program_test.start().await
}

#[tokio::test]
async fn run_test() {
    // Setup test
    let (banks, payer, blockhash) = setup().await;

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
    assert_eq!(oracles.items.len(), 0);

    // Submit add oracle transaction.
    let ix = (payer.pubkey(), 42);
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[&payer], blockhash);
    let res = banks.process_transaction(tx).await;
    assert!(res.is_ok());

    // // Verify counter was incremented.
    // let counter_account = banks.get_account(counter_address).await.unwrap().unwrap();
    // let counter = Counter::try_from_bytes(&counter_account.data).unwrap();
    // assert_eq!(counter.value, 42);
}
