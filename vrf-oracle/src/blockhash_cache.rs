use solana_client::nonblocking;
use solana_sdk::commitment_config::CommitmentConfig;
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
        let initial_blockhash = client
            .get_latest_blockhash_with_commitment(CommitmentConfig::processed())
            .await
            .unwrap()
            .0;
        let initial_slot = client
            .get_slot_with_commitment(CommitmentConfig::processed())
            .await
            .unwrap();
        let inner = Arc::new(RwLock::new(CacheData {
            blockhash: initial_blockhash,
            slot: initial_slot,
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
                    let latest = client
                        .get_latest_blockhash_with_commitment(CommitmentConfig::processed())
                        .await;
                    let slot = client
                        .get_slot_with_commitment(CommitmentConfig::processed())
                        .await;
                    if let (Ok(new_blockhash), Ok(new_slot)) = (latest, slot) {
                        let mut cache = inner.write().await;
                        cache.blockhash = new_blockhash.0;
                        cache.slot = new_slot;
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
        let initial_blockhash = self
            .client
            .get_latest_blockhash_with_commitment(CommitmentConfig::processed())
            .await;
        if let Ok(new_blockhash) = initial_blockhash {
            let mut cache = self.inner.write().await;
            cache.blockhash = new_blockhash.0;
            cache.timestamp = Instant::now();
        }
    }
}
