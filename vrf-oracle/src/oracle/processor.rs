use std::sync::Arc;

use anyhow::Result;
use ephemeral_vrf::vrf::{compute_vrf, verify_vrf};
use ephemeral_vrf_api::{
    prelude::{provide_randomness, QueueAccount, QueueItem},
    state::oracle_queue_pda,
    ID as PROGRAM_ID,
};
use log::info;
use solana_client::{rpc_client::RpcClient, rpc_config::RpcProgramAccountsConfig};
use solana_curve25519::{ristretto::PodRistrettoPoint, scalar::PodScalar};
use solana_sdk::{
    commitment_config::CommitmentConfig, instruction::AccountMeta, pubkey::Pubkey,
    signature::Signer, transaction::Transaction,
};

use crate::oracle::client::OracleClient;

pub async fn fetch_and_process_program_accounts(
    oracle_client: &Arc<OracleClient>,
    rpc_client: &Arc<RpcClient>,
    filters: Vec<solana_client::rpc_filter::RpcFilterType>,
) -> Result<()> {
    let config = RpcProgramAccountsConfig {
        account_config: solana_client::rpc_config::RpcAccountInfoConfig {
            commitment: Some(CommitmentConfig::processed()),
            encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
            ..Default::default()
        },
        filters: Some(filters),
        ..Default::default()
    };

    let accounts = rpc_client.get_program_accounts_with_config(&PROGRAM_ID, config)?;
    for (pubkey, acc) in accounts {
        if acc.owner == PROGRAM_ID {
            if let Ok(queue) = QueueAccount::try_from_bytes_with_discriminator(&acc.data) {
                process_oracle_queue(oracle_client, rpc_client, &pubkey, &queue).await;
            }
        }
    }
    Ok(())
}

pub async fn process_oracle_queue(
    oracle_client: &Arc<OracleClient>,
    rpc_client: &Arc<RpcClient>,
    queue: &Pubkey,
    oracle_queue: &QueueAccount,
) {
    if oracle_queue_pda(&oracle_client.keypair.pubkey(), oracle_queue.index).0 == *queue {
        if oracle_queue.item_count > 0 {
            info!(
                "Processing queue: {}, with len: {}",
                queue,
                oracle_queue.item_count
            );
        }

        for i in 0..oracle_queue.items.len() {
            // Check if this slot has a valid item
            if let Some(item) = &oracle_queue.items[i] {
                let input_seed = item.id;
                let mut attempts = 0;
                while attempts < 5 {
                    match ProcessableItem(item.clone())
                        .process_item(oracle_client, rpc_client, &input_seed, queue)
                        .await
                    {
                        Ok(signature) => {
                            println!("Transaction signature: {signature}");
                            break;
                        }
                        Err(e) => {
                            attempts += 1;
                            println!("Failed to send transaction: {e:?}")
                        }
                    }
                }
            }
        }
    }
}

#[repr(transparent)]
pub struct ProcessableItem(pub QueueItem);

impl ProcessableItem {
    pub async fn process_item(
        &self,
        oracle_client: &OracleClient,
        rpc_client: &Arc<RpcClient>,
        vrf_input: &[u8; 32],
        queue: &Pubkey,
    ) -> Result<String> {
        let (output, (commitment_base, commitment_hash, s)) =
            compute_vrf(oracle_client.oracle_vrf_sk, vrf_input);

        assert!(verify_vrf(
            oracle_client.oracle_vrf_pk,
            vrf_input,
            output,
            (commitment_base, commitment_hash, s),
        ));

        let mut ix = provide_randomness(
            oracle_client.keypair.pubkey(),
            *queue,
            self.0.callback_program_id,
            *vrf_input,
            PodRistrettoPoint(output.to_bytes()),
            PodRistrettoPoint(commitment_base.to_bytes()),
            PodRistrettoPoint(commitment_hash.to_bytes()),
            PodScalar(s.to_bytes()),
        );

        ix.accounts
            .extend(self.0.callback_accounts_meta.iter().map(|a| AccountMeta {
                pubkey: a.pubkey,
                is_signer: a.is_signer,
                is_writable: a.is_writable,
            }));

        let blockhash = rpc_client
            .get_latest_blockhash_with_commitment(CommitmentConfig::processed())?
            .0;
        let tx = Transaction::new_signed_with_payer(
            &[
                solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
                    200_000,
                ),
                ix,
            ],
            Some(&oracle_client.keypair.pubkey()),
            &[&oracle_client.keypair],
            blockhash,
        );

        Ok(rpc_client.send_and_confirm_transaction(&tx)?.to_string())
    }
}
