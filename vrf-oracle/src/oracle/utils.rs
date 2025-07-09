use ephemeral_vrf_api::prelude::QueueAccount;
use ephemeral_vrf_api::state::AccountWithDiscriminator;
use solana_client::rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType};

pub fn queue_memcmp_filter() -> Vec<RpcFilterType> {
    vec![RpcFilterType::Memcmp(Memcmp::new(
        0,
        MemcmpEncodedBytes::Bytes(QueueAccount::discriminator().to_bytes().to_vec()),
    ))]
}
