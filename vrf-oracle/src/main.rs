mod args;
mod blockhash_cache;
mod oracle;

use crate::oracle::client::OracleClient;
use anyhow::Result;
use args::Args;
use clap::Parser;
use solana_sdk::signature::Keypair;
use std::sync::Arc;

pub const DEFAULT_IDENTITY: &str =
    "D4fURjsRpMj1vzfXqHgL94UeJyXR8DFyfyBDmbY647PnpuDzszvbRocMQu6Tzr1LUzBTQvXjarCxeb94kSTqvYx";

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
                eprintln!("Oracle crashed: {e}");
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
        }
    }
}
