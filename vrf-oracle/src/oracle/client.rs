use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use solana_client::{
    pubsub_client::PubsubClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Keypair};
use tokio::sync::RwLock;

use helius_laserstream::{
    grpc::{
        SubscribeRequest, SubscribeRequestFilterAccounts, SubscribeRequestFilterAccountsFilter,
        SubscribeRequestFilterAccountsFilterMemcmp,
    },
    subscribe, AccountsFilterMemcmpOneof, AccountsFilterOneof, LaserstreamConfig,
};

use crate::blockhash_cache::BlockhashCache;
use crate::oracle::processor::{fetch_and_process_program_accounts, process_oracle_queue};
use crate::oracle::sources::{LaserstreamSource, WebSocketSource};
use crate::oracle::utils::queue_memcmp_filter;
use curve25519_dalek::{RistrettoPoint, Scalar};
use ephemeral_vrf::vrf::generate_vrf_keypair;
use ephemeral_vrf_api::prelude::AccountDiscriminator;
use ephemeral_vrf_api::{prelude::Queue, ID as PROGRAM_ID};
use log::{error, info, warn};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::signer::Signer;

pub struct OracleClient {
    pub keypair: Keypair,
    pub rpc_url: String,
    pub websocket_url: String,
    pub oracle_vrf_sk: Scalar,
    pub oracle_vrf_pk: RistrettoPoint,
    pub laserstream_api_key: Option<String>,
    pub laserstream_endpoint: Option<String>,
    pub queue_stats: Arc<RwLock<HashMap<String, usize>>>,
}

#[async_trait]
pub trait QueueUpdateSource: Send {
    async fn next(&mut self) -> Option<(Pubkey, Queue)>;
}

impl OracleClient {
    pub fn new(
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
            queue_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn run(self: Arc<Self>) -> Result<()> {
        info!(
            "Starting VRF Oracle with public key: {}",
            self.keypair.pubkey()
        );
        let rpc_client = Arc::new(RpcClient::new_with_commitment(
            self.rpc_url.clone(),
            CommitmentConfig::processed(),
        ));
        fetch_and_process_program_accounts(&self, &rpc_client, queue_memcmp_filter()).await?;
        let blockhash_cache = Arc::new(BlockhashCache::new(Arc::clone(&rpc_client)).await);

        // Periodically refresh and process program accounts every 30 seconds
        {
            let self_clone = Arc::clone(&self);
            let rpc_client_clone = Arc::clone(&rpc_client);
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
                loop {
                    interval.tick().await;
                    if let Err(err) = fetch_and_process_program_accounts(
                        &self_clone,
                        &rpc_client_clone,
                        queue_memcmp_filter(),
                    )
                    .await
                    {
                        error!("Periodic fetch_and_process_program_accounts failed: {err:?}");
                    }
                }
            });
        }

        loop {
            match self.create_update_source().await {
                Ok(mut source) => {
                    info!("Update source connected successfully");
                    while let Some((pubkey, queue)) = source.next().await {
                        process_oracle_queue(&self, &rpc_client, &blockhash_cache, &pubkey, &queue)
                            .await;
                    }
                    drop(source);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    warn!("Update source stream ended. Attempting to reconnect...");
                }
                Err(err) => {
                    error!("Failed to create update source: {err:?}. Retrying in 5 seconds...");
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn create_update_source(self: &Arc<Self>) -> Result<Box<dyn QueueUpdateSource>> {
        if let (Some(api_key), Some(endpoint)) =
            (&self.laserstream_api_key, &self.laserstream_endpoint)
        {
            info!("Connecting to gRPC: {endpoint}");
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
                                    AccountDiscriminator::Queue.to_bytes().to_vec(),
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
            info!("Connecting to WebSocket: {}", self.websocket_url);
            let config = RpcProgramAccountsConfig {
                account_config: RpcAccountInfoConfig {
                    commitment: Some(CommitmentConfig::processed()),
                    encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
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
