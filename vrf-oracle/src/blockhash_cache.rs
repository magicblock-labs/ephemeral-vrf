use serde_json::json;
use solana_client::nonblocking;
use solana_client::rpc_request::RpcRequest;
use solana_client::rpc_response::{Response, RpcBlockhash};
use solana_commitment_config::CommitmentConfig;
use solana_sdk::hash::Hash;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct BlockhashCache {
    inner: Arc<RwLock<CacheData>>,
    client: Arc<nonblocking::rpc_client::RpcClient>,
}

struct CacheData {
    blockhash: Hash,
    slot: u64,
    timestamp: Instant,
}

impl BlockhashCache {
    pub async fn new(client: Arc<nonblocking::rpc_client::RpcClient>) -> Self {
        let (blockhash, slot) = Self::fetch_blockhash_and_slot(&client).await.unwrap();
        let inner = Arc::new(RwLock::new(CacheData {
            blockhash,
            slot,
            timestamp: Instant::now(),
        }));

        let cache = Self { inner, client };

        cache.spawn_refresh_task();
        cache
    }

    fn spawn_refresh_task(&self) {
        let inner = self.inner.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;

                let should_refresh = {
                    let cache = inner.read().await;
                    cache.timestamp.elapsed() > Duration::from_secs(60)
                };

                if should_refresh {
                    if let Ok((blockhash, slot)) = Self::fetch_blockhash_and_slot(&client).await {
                        let mut cache = inner.write().await;
                        cache.blockhash = blockhash;
                        cache.slot = slot;
                        cache.timestamp = Instant::now();
                    }
                }
            }
        });
    }

    #[allow(dead_code)]
    pub async fn get_blockhash(&self) -> Hash {
        let cache = self.inner.read().await;
        cache.blockhash
    }

    pub async fn get_blockhash_and_slot(&self) -> (Hash, u64) {
        let cache = self.inner.read().await;
        (cache.blockhash, cache.slot)
    }

    pub async fn refresh_blockhash(&self) {
        if let Ok((blockhash, slot)) = Self::fetch_blockhash_and_slot(&self.client).await {
            let mut cache = self.inner.write().await;
            cache.blockhash = blockhash;
            cache.slot = slot;
            cache.timestamp = Instant::now();
        }
    }

    async fn fetch_blockhash_and_slot(
        client: &nonblocking::rpc_client::RpcClient,
    ) -> anyhow::Result<(Hash, u64)> {
        let resp: Response<RpcBlockhash> = client
            .send(
                RpcRequest::GetLatestBlockhash,
                json!([CommitmentConfig::processed()]),
            )
            .await?;
        let blockhash = resp.value.blockhash.parse()?;
        Ok((blockhash, resp.context.slot))
    }
}
