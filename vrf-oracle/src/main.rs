use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use crossbeam_channel::Receiver;
use curve25519_dalek::{RistrettoPoint, Scalar};
use ephemeral_vrf::vrf::{compute_vrf, generate_vrf_keypair, verify_vrf};
use ephemeral_vrf_api::{
    prelude::{provide_randomness, QueueAccount, QueueItem},
    state::{oracle_queue_pda, AccountWithDiscriminator},
    ID as PROGRAM_ID,
};
use futures_util::StreamExt;
use helius_laserstream::{
    grpc::{
        subscribe_update::UpdateOneof, SubscribeRequest, SubscribeRequestFilterAccounts,
        SubscribeRequestFilterAccountsFilter, SubscribeRequestFilterAccountsFilterMemcmp,
        SubscribeUpdate,
    },
    subscribe, AccountsFilterMemcmpOneof, AccountsFilterOneof, LaserstreamConfig, LaserstreamError,
};
use solana_account_decoder::UiAccountEncoding;
use solana_client::pubsub_client::PubsubProgramClientSubscription;
use solana_client::rpc_response::{Response, RpcKeyedAccount};
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
use std::pin::Pin;
use std::{collections::HashMap, str::FromStr, sync::Arc};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(long, env = "VRF_ORACLE_IDENTITY")]
    identity: Option<String>,

    #[arg(long, env = "RPC_URL", default_value = "http://localhost:8899")]
    rpc_url: String,

    #[arg(long, env = "WEBSOCKET_URL", default_value = "ws://localhost:8900")]
    websocket_url: String,

    #[arg(long, env = "LASERSTREAM_API_KEY")]
    laserstream_api_key: Option<String>,

    #[arg(long, env = "LASERSTREAM_ENDPOINT")]
    laserstream_endpoint: Option<String>,
}

struct OracleClient {
    keypair: Keypair,
    rpc_url: String,
    websocket_url: String,
    oracle_vrf_sk: Scalar,
    oracle_vrf_pk: RistrettoPoint,
    laserstream_api_key: Option<String>,
    laserstream_endpoint: Option<String>,
}

#[async_trait]
trait QueueUpdateSource: Send {
    async fn next(&mut self) -> Option<(Pubkey, QueueAccount)>;
}

struct WebSocketSource {
    subscription: Receiver<Response<RpcKeyedAccount>>,
    #[allow(dead_code)]
    client: PubsubProgramClientSubscription,
}

impl Drop for WebSocketSource {
    fn drop(&mut self) {
        let _ = self.client.shutdown();
    }
}

#[async_trait]
impl QueueUpdateSource for WebSocketSource {
    async fn next(&mut self) -> Option<(Pubkey, QueueAccount)> {
        let update = self.subscription.recv().ok()?;
        let data = update.value.account.data.decode()?;
        if update.value.account.owner != PROGRAM_ID.to_string() {
            return None;
        }
        let queue = QueueAccount::try_from_bytes_with_discriminator(&data).ok()?;
        let pubkey = Pubkey::from_str(&update.value.pubkey).ok()?;
        Some((pubkey, queue))
    }
}

struct LaserstreamSource {
    stream:
        Pin<Box<dyn futures_core::Stream<Item = Result<SubscribeUpdate, LaserstreamError>> + Send>>,
}

#[async_trait]
impl QueueUpdateSource for LaserstreamSource {
    async fn next(&mut self) -> Option<(Pubkey, QueueAccount)> {
        while let Some(result) = self.stream.next().await {
            let update = result.ok()?;
            if let Some(UpdateOneof::Account(acc)) = update.update_oneof {
                let acc = acc.account?;
                let queue = QueueAccount::try_from_bytes_with_discriminator(&acc.data).ok()?;
                let pubkey = Pubkey::new_from_array(acc.pubkey.try_into().ok()?);
                return Some((pubkey, queue));
            }
        }
        None
    }
}

impl OracleClient {
    fn new(
        keypair: Keypair,
        rpc_url: String,
        websocket_url: String,
        laserstream_endpoint: Option<String>,
        laserstream_api_key: Option<String>,
    ) -> Self {
        let (oracle_vrf_sk, oracle_vrf_pk) = generate_vrf_keypair(&keypair);
        Self {
            keypair,
            rpc_url,
            websocket_url,
            oracle_vrf_sk,
            oracle_vrf_pk,
            laserstream_api_key,
            laserstream_endpoint,
        }
    }

    async fn run(self: Arc<Self>) -> Result<()> {
        let rpc_client = Arc::new(RpcClient::new_with_commitment(
            &self.rpc_url,
            CommitmentConfig::processed(),
        ));
        fetch_and_process_program_accounts(&self, &rpc_client, queue_memcmp_filter()).await?;
        let mut source = self.create_update_source().await?;
        while let Some((pubkey, queue)) = source.next().await {
            process_oracle_queue(&self, &rpc_client, &pubkey, &queue).await;
        }
        Ok(())
    }

    async fn create_update_source(self: &Arc<Self>) -> Result<Box<dyn QueueUpdateSource>> {
        if let (Some(api_key), Some(endpoint)) =
            (&self.laserstream_api_key, &self.laserstream_endpoint)
        {
            let config = LaserstreamConfig {
                api_key: api_key.clone(),
                endpoint: endpoint.parse()?,
                ..Default::default()
            };

            let mut filters = HashMap::new();
            filters.insert(
                "oracle".to_string(),
                SubscribeRequestFilterAccounts {
                    owner: vec![PROGRAM_ID.to_string()],
                    filters: vec![SubscribeRequestFilterAccountsFilter {
                        filter: Some(AccountsFilterOneof::Memcmp(
                            SubscribeRequestFilterAccountsFilterMemcmp {
                                offset: 0,
                                data: Some(AccountsFilterMemcmpOneof::Bytes(
                                    QueueAccount::discriminator().to_bytes().to_vec(),
                                )),
                            },
                        )),
                    }],
                    ..Default::default()
                },
            );

            let stream = subscribe(
                config,
                SubscribeRequest {
                    accounts: filters,
                    ..Default::default()
                },
            );
            Ok(Box::new(LaserstreamSource {
                stream: Box::pin(stream),
            }))
        } else {
            let config = RpcProgramAccountsConfig {
                account_config: RpcAccountInfoConfig {
                    commitment: Some(CommitmentConfig::processed()),
                    encoding: Some(UiAccountEncoding::Base64),
                    ..Default::default()
                },
                filters: Some(queue_memcmp_filter()),
                ..Default::default()
            };
            let (client, sub) =
                PubsubClient::program_subscribe(&self.websocket_url, &PROGRAM_ID, Some(config))?;
            Ok(Box::new(WebSocketSource {
                client,
                subscription: sub,
            }))
        }
    }
}

fn queue_memcmp_filter() -> Vec<RpcFilterType> {
    vec![RpcFilterType::Memcmp(Memcmp::new(
        0,
        MemcmpEncodedBytes::Bytes(QueueAccount::discriminator().to_bytes().to_vec()),
    ))]
}

async fn fetch_and_process_program_accounts(
    oracle_client: &Arc<OracleClient>,
    rpc_client: &Arc<RpcClient>,
    filters: Vec<RpcFilterType>,
) -> Result<()> {
    let config = RpcProgramAccountsConfig {
        account_config: RpcAccountInfoConfig {
            commitment: Some(CommitmentConfig::processed()),
            encoding: Some(UiAccountEncoding::Base64),
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

async fn process_oracle_queue(
    oracle_client: &Arc<OracleClient>,
    rpc_client: &Arc<RpcClient>,
    queue: &Pubkey,
    oracle_queue: &QueueAccount,
) {
    if oracle_queue_pda(&oracle_client.keypair.pubkey(), oracle_queue.index).0 != *queue {
        return;
    }

    for (seed, item) in oracle_queue.items.iter() {
        let mut attempts = 0;
        while attempts < MAX_ATTEMPTS {
            match ProcessableItem(item.clone())
                .process_item(oracle_client, rpc_client, seed, queue)
                .await
            {
                Ok(sig) => {
                    println!("Transaction: {}", sig);
                    break;
                }
                Err(e) => {
                    attempts += 1;
                    println!("Retry {}/5 failed: {}", attempts, e);
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

        let blockhash = rpc_client
            .get_latest_blockhash_with_commitment(CommitmentConfig::processed())?
            .0;
        let tx = Transaction::new_signed_with_payer(
            &[
                ComputeBudgetInstruction::set_compute_unit_limit(200_000),
                ix,
            ],
            Some(&oracle_client.keypair.pubkey()),
            &[&oracle_client.keypair],
            blockhash,
        );

        Ok(rpc_client.send_and_confirm_transaction(&tx)?.to_string())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    let identity = args
        .identity
        .unwrap_or_else(|| DEFAULT_IDENTITY.to_string());
    let keypair = Keypair::from_base58_string(&identity);
    let oracle = Arc::new(OracleClient::new(
        keypair,
        args.rpc_url,
        args.websocket_url,
        args.laserstream_endpoint,
        args.laserstream_api_key,
    ));

    loop {
        match Arc::clone(&oracle).run().await {
            Ok(_) => continue,
            Err(e) => {
                eprintln!("Oracle crashed: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
        }
    }
}

const DEFAULT_IDENTITY: &str =
    "D4fURjsRpMj1vzfXqHgL94UeJyXR8DFyfyBDmbY647PnpuDzszvbRocMQu6Tzr1LUzBTQvXjarCxeb94kSTqvYx";
const MAX_ATTEMPTS: u8 = 5;
