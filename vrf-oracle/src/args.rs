use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Args {
    #[arg(long, env = "VRF_ORACLE_IDENTITY")]
    pub identity: Option<String>,

    #[arg(long, env = "RPC_URL", default_value = "http://localhost:8899")]
    pub rpc_url: String,

    #[arg(long, env = "WEBSOCKET_URL", default_value = "ws://localhost:8900")]
    pub websocket_url: String,

    #[arg(long, env = "LASERSTREAM_API_KEY")]
    pub laserstream_api_key: Option<String>,

    #[arg(long, env = "LASERSTREAM_ENDPOINT")]
    pub laserstream_endpoint: Option<String>,

    #[arg(long, env = "VRF_ORACLE_HTTP_PORT")]
    pub http_port: Option<u16>,
}
