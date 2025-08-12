use std::sync::Arc;

use crate::oracle::client::OracleClient;
use anyhow::Result;
use ephemeral_vrf::vrf::{compute_vrf, verify_vrf};
use ephemeral_vrf_api::{
    prelude::{provide_randomness, purge_expired_requests, Queue, QueueItem, QUEUE_TTL_SLOTS},
    state::oracle_queue_pda,
    ID as PROGRAM_ID,
};
use log::info;
use solana_client::{rpc_client::RpcClient, rpc_config::RpcProgramAccountsConfig};
use solana_curve25519::{ristretto::PodRistrettoPoint, scalar::PodScalar};
use solana_sdk::{
    commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signer,
    transaction::Transaction,
};
use steel::AccountDeserialize;

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
            if let Ok(queue) = Queue::try_from_bytes(&acc.data) {
                process_oracle_queue(oracle_client, rpc_client, &pubkey, queue).await;
            }
        }
    }
    Ok(())
}

pub async fn process_oracle_queue(
    oracle_client: &Arc<OracleClient>,
    rpc_client: &Arc<RpcClient>,
    queue: &Pubkey,
    oracle_queue: &Queue,
) {
    if oracle_queue_pda(&oracle_client.keypair.pubkey(), oracle_queue.index).0 == *queue {
        if oracle_queue.item_count > 0 {
            info!(
                "Processing queue: {}, with len: {}",
                queue, oracle_queue.item_count
            );
        }

        for item in oracle_queue.iter_items() {
            // Check if this slot has a valid item
            let input_seed = item.id;
            let mut attempts = 0;
            while attempts < 5 {
                match ProcessableItem(*item)
                    .process_item(
                        oracle_client,
                        rpc_client,
                        &input_seed,
                        queue,
                        oracle_queue.index,
                    )
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

#[repr(transparent)]
pub struct ProcessableItem(pub QueueItem);

impl ProcessableItem {
    pub async fn process_item(
        &self,
        oracle_client: &OracleClient,
        rpc_client: &Arc<RpcClient>,
        vrf_input: &[u8; 32],
        queue: &Pubkey,
        queue_index: u8,
    ) -> Result<String> {
        // Check expiry against current slot
        let current_slot = rpc_client.get_slot_with_commitment(CommitmentConfig::processed())?;
        let age = current_slot.saturating_sub(self.0.slot);
        let budget = match self.0.priority_request {
            1 => 200_000,
            _ => 180_000,
        };

        if age > QUEUE_TTL_SLOTS {
            // Request expired: purge instead of providing randomness
            let ix = purge_expired_requests(oracle_client.keypair.pubkey(), queue_index);
            let blockhash = rpc_client
                .get_latest_blockhash_with_commitment(CommitmentConfig::processed())?
                .0;
            let tx = Transaction::new_signed_with_payer(
                &[
                    solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
                        budget,
                    ),
                    ix,
                ],
                Some(&oracle_client.keypair.pubkey()),
                &[&oracle_client.keypair],
                blockhash,
            );
            return Ok(rpc_client.send_and_confirm_transaction(&tx)?.to_string());
        }

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
            Pubkey::new_from_array(self.0.callback_program_id),
            *vrf_input,
            PodRistrettoPoint(output.to_bytes()),
            PodRistrettoPoint(commitment_base.to_bytes()),
            PodRistrettoPoint(commitment_hash.to_bytes()),
            PodScalar(s.to_bytes()),
        );

        ix.accounts.extend(
            self.0
                .callback_accounts_meta
                .iter()
                .map(|a| a.to_account_meta()),
        );

        let blockhash = rpc_client
            .get_latest_blockhash_with_commitment(CommitmentConfig::processed())?
            .0;
        let tx = Transaction::new_signed_with_payer(
            &[
                solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
                    budget,
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
