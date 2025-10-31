mod args;
mod blockhash_cache;
mod oracle;

use crate::oracle::client::OracleClient;
use anyhow::Result;
use args::Args;
use clap::Parser;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server, StatusCode,
};
use log::info;
use solana_sdk::signature::Keypair;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

pub const DEFAULT_IDENTITY: &str =
    "D4fURjsRpMj1vzfXqHgL94UeJyXR8DFyfyBDmbY647PnpuDzszvbRocMQu6Tzr1LUzBTQvXjarCxeb94kSTqvYx";

async fn start_http_server(oracle: Arc<OracleClient>, port: u16) -> Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let make_svc = make_service_fn(move |_| {
        let oracle = Arc::clone(&oracle);
        async move {
            Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                let oracle = Arc::clone(&oracle);
                async move {
                    if req.method() == Method::GET && req.uri().path() == "/queues" {
                        let stats = oracle.queue_stats.read().await;
                        let body =
                            serde_json::to_string(&*stats).unwrap_or_else(|_| "{}".to_string());
                        Ok::<_, Infallible>(Response::new(Body::from(body)))
                    } else {
                        let mut not_found = Response::new(Body::from("Not Found"));
                        *not_found.status_mut() = StatusCode::NOT_FOUND;
                        Ok::<_, Infallible>(not_found)
                    }
                }
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);
    info!(
        "HTTP server listening on 0.0.0.0:{} (try: curl http://localhost:{}/queues)",
        port, port
    );
    tokio::spawn(async move {
        if let Err(e) = server.await {
            eprintln!("HTTP server error: {}", e);
        }
    });
    Ok(())
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

    // Start minimal HTTP server exposing /queues
    start_http_server(Arc::clone(&oracle), args.http_port).await?;

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
