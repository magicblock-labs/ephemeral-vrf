use ephemeral_vrf_api::prelude::AccountDiscriminator;
use solana_client::rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType};

pub fn queue_memcmp_filter() -> Vec<RpcFilterType> {
    vec![RpcFilterType::Memcmp(Memcmp::new(
        0,
        MemcmpEncodedBytes::Bytes(AccountDiscriminator::Queue.to_bytes().to_vec()),
    ))]
}
