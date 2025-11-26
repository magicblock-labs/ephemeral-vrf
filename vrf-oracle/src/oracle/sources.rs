use std::{pin::Pin, str::FromStr};

use async_trait::async_trait;
use crossbeam_channel::Receiver;
use futures_util::StreamExt;
use solana_client::{
    pubsub_client::PubsubProgramClientSubscription, rpc_response::RpcKeyedAccount,
};
use solana_sdk::pubkey::Pubkey;

use crate::oracle::client::QueueUpdateSource;
use ephemeral_vrf_api::prelude::Queue;
use ephemeral_vrf_api::ID as PROGRAM_ID;
use helius_laserstream::{
    grpc::{subscribe_update::UpdateOneof, SubscribeUpdate},
    LaserstreamError,
};

pub struct WebSocketSource {
    pub subscription: Receiver<solana_client::rpc_response::Response<RpcKeyedAccount>>,
    pub client: PubsubProgramClientSubscription,
}

impl Drop for WebSocketSource {
    fn drop(&mut self) {
        let _ = self.client.shutdown();
    }
}

#[async_trait]
impl QueueUpdateSource for WebSocketSource {
    async fn next(&mut self) -> Option<(Pubkey, Queue, u64)> {
        let update = self.subscription.recv().ok()?;
        let data = update.value.account.data.decode()?;
        if update.value.account.owner != PROGRAM_ID.to_string() {
            return None;
        }
        let queue = Queue::try_from_bytes(&data).ok()?;
        let pubkey = Pubkey::from_str(&update.value.pubkey).ok()?;
        Some((pubkey, *queue, update.context.slot))
    }
}

pub struct LaserstreamSource {
    pub stream:
        Pin<Box<dyn futures_core::Stream<Item = Result<SubscribeUpdate, LaserstreamError>> + Send>>,
}

#[async_trait]
impl QueueUpdateSource for LaserstreamSource {
    async fn next(&mut self) -> Option<(Pubkey, Queue, u64)> {
        while let Some(result) = self.stream.next().await {
            let update = result.ok()?;
            if let Some(UpdateOneof::Account(acc)) = update.update_oneof {
                let slot = acc.slot;
                let acc = acc.account?;
                let queue = Queue::try_from_bytes(&acc.data).ok()?;
                let pubkey = Pubkey::new_from_array(acc.pubkey.try_into().ok()?);
                return Some((pubkey, *queue, slot));
            }
        }
        None
    }
}
