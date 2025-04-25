use anyhow::{anyhow, Result};
use clap::Parser;
use curve25519_dalek::{RistrettoPoint, Scalar};
use ephemeral_vrf::vrf::{compute_vrf, generate_vrf_keypair, verify_vrf};
use ephemeral_vrf_api::prelude::{provide_randomness, QueueAccount, QueueItem};
use ephemeral_vrf_api::state::oracle_queue_pda;
use log::info;
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    pubsub_client::PubsubClient,
    rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
};
use solana_curve25519::{ristretto::PodRistrettoPoint, scalar::PodScalar};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    compute_budget::ComputeBudgetInstruction,
    instruction::AccountMeta,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::str::FromStr;
use std::sync::Arc;

/// Maximum number of retry attempts for failed transactions
const MAX_RETRY_ATTEMPTS: u8 = 5;

/// VRF Oracle client
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long, env = "VRF_ORACLE_IDENTITY")]
    identity: Option<String>,

    #[arg(short, long, env = "RPC_URL", default_value = "http://localhost:8899")]
    rpc_url: String,

    #[arg(
        short,
        long,
        env = "WEBSOCKET_URL",
        default_value = "ws://localhost:8900"
    )]
    websocket_url: String,
}

struct OracleClient {
    keypair: Keypair,
    rpc_url: String,
    websocket_url: String,
    oracle_vrf_sk: Scalar,
    oracle_vrf_pk: RistrettoPoint,
}

const DEFAULT_IDENTITY: &str =
    "D4fURjsRpMj1vzfXqHgL94UeJyXR8DFyfyBDmbY647PnpuDzszvbRocMQu6Tzr1LUzBTQvXjarCxeb94kSTqvYx";

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    let identity = args
        .identity
        .unwrap_or_else(|| DEFAULT_IDENTITY.to_string());
    let keypair = Keypair::from_base58_string(&identity);
    let oracle = OracleClient::new(keypair, args.rpc_url, args.websocket_url);

    loop {
        match oracle.run().await {
            Ok(_) => continue,
            Err(e) => {
                eprintln!("Oracle crashed with error: {:?}. Restarting...", e);
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
        }
    }
}

impl OracleClient {
    fn new(keypair: Keypair, rpc_url: String, websocket_url: String) -> Self {
        let (oracle_vrf_sk, oracle_vrf_pk) = generate_vrf_keypair(&keypair);
        Self {
            keypair,
            rpc_url,
            websocket_url,
            oracle_vrf_sk,
            oracle_vrf_pk,
        }
    }

    async fn run(&self) -> Result<()> {
        info!(
            "Starting VRF Oracle with public key: {}",
            self.keypair.pubkey()
        );
        info!("Connecting to RPC: {}", self.rpc_url);
        info!("Connecting to WebSocket: {}", self.websocket_url);

        let rpc_client = Arc::new(RpcClient::new_with_commitment(
            &self.rpc_url,
            CommitmentConfig::processed(),
        ));

        let filters = vec![RpcFilterType::Memcmp(Memcmp::new(
            0,
            MemcmpEncodedBytes::Bytes(vec![3, 0, 0, 0, 0, 0, 0, 0]),
        ))];

        let program_config = RpcProgramAccountsConfig {
            account_config: RpcAccountInfoConfig {
                commitment: Some(CommitmentConfig::processed()),
                encoding: Some(UiAccountEncoding::Base64),
                ..Default::default()
            },
            filters: Some(filters.clone()),
            ..Default::default()
        };

        let (mut client, subscription) = PubsubClient::program_subscribe(
            &self.websocket_url,
            &ephemeral_vrf_api::ID,
            Some(program_config),
        )?;

        fetch_and_process_program_accounts(self, &rpc_client, filters).await?;

        while let Ok(update) = subscription.recv() {
            if let Some(data) = update.value.account.data.decode() {
                if update.value.account.owner == ephemeral_vrf_api::ID.to_string() {
                    if let Ok(oracle_queue) = QueueAccount::try_from_bytes_with_discriminator(&data)
                    {
                        let pubkey = Pubkey::from_str(&update.value.pubkey).unwrap_or_default();
                        process_oracle_queue(self, &rpc_client, &pubkey, &oracle_queue).await;
                    }
                }
            }
        }

        client
            .shutdown()
            .map_err(|_| anyhow!("Invalid state: failed to shutdown client"))?;
        Ok(())
    }
}

async fn fetch_and_process_program_accounts(
    oracle_client: &OracleClient,
    rpc_client: &Arc<RpcClient>,
    filters: Vec<RpcFilterType>,
) -> Result<()> {
    let program_config = RpcProgramAccountsConfig {
        account_config: RpcAccountInfoConfig {
            commitment: Some(CommitmentConfig::processed()),
            encoding: Some(UiAccountEncoding::Base64),
            ..Default::default()
        },
        filters: Some(filters),
        ..Default::default()
    };

    let accounts =
        rpc_client.get_program_accounts_with_config(&ephemeral_vrf_api::ID, program_config)?;

    for (queue_pubkey, queue_account) in accounts {
        if queue_account.owner == ephemeral_vrf_api::ID {
            if let Ok(oracle_queue) =
                QueueAccount::try_from_bytes_with_discriminator(&queue_account.data)
            {
                process_oracle_queue(oracle_client, rpc_client, &queue_pubkey, &oracle_queue).await;
            }
        }
    }

    Ok(())
}

async fn process_oracle_queue(
    oracle_client: &OracleClient,
    rpc_client: &Arc<RpcClient>,
    queue: &Pubkey,
    oracle_queue: &QueueAccount,
) {
    if oracle_queue_pda(&oracle_client.keypair.pubkey(), oracle_queue.index).0 == *queue {
        if !oracle_queue.items.is_empty() {
            info!(
                "Processing queue: {}, with len: {}",
                queue,
                oracle_queue.items.len()
            );
        }

        for (input_seed, item) in oracle_queue.items.iter() {
            let mut attempts = 0;
            while attempts < MAX_RETRY_ATTEMPTS {
                match ProcessableItem(item.clone())
                    .process_item(oracle_client, rpc_client, input_seed, queue)
                    .await
                {
                    Ok(signature) => {
                        println!("Transaction signature: {}", signature);
                        break;
                    }
                    Err(e) => {
                        attempts += 1;
                        println!("Failed to send transaction: {:?}", e)
                    }
                }
            }
        }
    }
}

#[repr(transparent)]
struct ProcessableItem(QueueItem);

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

        let compute_ix = ComputeBudgetInstruction::set_compute_unit_limit(200_000);
        let blockhash = rpc_client
            .get_latest_blockhash_with_commitment(CommitmentConfig::processed())?
            .0;

        let tx = Transaction::new_signed_with_payer(
            &[compute_ix, ix],
            Some(&oracle_client.keypair.pubkey()),
            &[&oracle_client.keypair],
            blockhash,
        );

        Ok(rpc_client.send_and_confirm_transaction(&tx)?.to_string())
    }
}
