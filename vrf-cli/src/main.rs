use anyhow::Result;
use clap::{Parser, Subcommand};
use ephemeral_vrf::vrf::generate_vrf_keypair;
use ephemeral_vrf_api::prelude::*;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    compute_budget::ComputeBudgetInstruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::process::exit;
use std::str::FromStr;
use steel::AccountDeserialize;

/// VRF CLI - A tool to interact with the Ephemeral VRF program
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Solana RPC URL
    #[arg(short, long, env = "RPC_URL", default_value = "http://localhost:8899")]
    rpc_url: String,

    /// Base58 encoded keypair
    #[arg(short, long, env = "KEYPAIR_BASE58")]
    keypair: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize the program state
    Initialize {},

    /// Add an oracle
    AddOracle {
        /// Oracle identity pubkey
        #[arg(short, long)]
        identity: String,

        /// Oracle pubkey
        #[arg(short, long)]
        oracle_pubkey: String,
    },

    /// Remove an oracle
    RemoveOracle {
        /// Oracle identity pubkey
        #[arg(short, long)]
        identity: String,
    },

    /// Initialize an oracle queue
    InitializeOracleQueue {
        /// Oracle identity pubkey
        #[arg(short, long)]
        identity: String,

        /// Queue index
        #[arg(long)]
        index: u8,
    },

    /// Delegate an oracle queue
    DelegateOracleQueue {
        /// Queue pubkey
        #[arg(short, long)]
        queue: String,
    },

    /// Undelegate an oracle queue
    UndelegateOracleQueue {
        /// Queue pubkey
        #[arg(short, long)]
        queue: String,
    },

    /// Close an oracle queue
    CloseOracleQueue {
        /// Queue pubkey
        #[arg(short, long)]
        queue: String,
    },

    /// Derive the current oracle pubkey for the given identity.
    DerivePubkey {},

    /// List all existing oracle's queues.
    ListQueue {},
}

fn get_signer(keypair: &str) -> Keypair {
    Keypair::from_base58_string(keypair)
}

const DEFAULT_IDENTITY: &str =
    "D4fURjsRpMj1vzfXqHgL94UeJyXR8DFyfyBDmbY647PnpuDzszvbRocMQu6Tzr1LUzBTQvXjarCxeb94kSTqvYx";

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let rpc_client = RpcClient::new_with_commitment(&args.rpc_url, CommitmentConfig::confirmed());
    let signer = get_signer(
        args.keypair
            .unwrap_or(DEFAULT_IDENTITY.to_string())
            .as_str(),
    );

    println!("Using signer: {}", signer.pubkey());
    println!("Rpc: {}", args.rpc_url);

    let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(200_000);
    let blockhash = rpc_client.get_latest_blockhash()?;

    let instructions = match &args.command {
        Commands::Initialize {} => {
            println!("Initializing program state...");
            vec![initialize(signer.pubkey())]
        }
        Commands::AddOracle {
            identity,
            oracle_pubkey,
        } => {
            let identity = Pubkey::from_str(identity)?;
            let oracle_pubkey_bytes = Pubkey::from_str(oracle_pubkey)?.to_bytes();
            println!("Adding oracle with identity: {identity}");
            vec![add_oracle(signer.pubkey(), identity, oracle_pubkey_bytes)]
        }
        Commands::RemoveOracle { identity } => {
            let identity = Pubkey::from_str(identity)?;
            println!("Removing oracle with identity: {identity}");
            vec![remove_oracle(signer.pubkey(), identity)]
        }
        Commands::InitializeOracleQueue { identity, index } => {
            let identity = Pubkey::from_str(identity)?;
            println!("Initializing oracle queue for identity: {identity} with index: {index}");
            initialize_oracle_queue(signer.pubkey(), identity, *index).to_vec()
        }
        Commands::DelegateOracleQueue { queue } => {
            let queue = Pubkey::from_str(queue)?;
            let queue_account = rpc_client.get_account(&queue)?;
            let queue_struct = Queue::try_from_bytes(queue_account.data.as_slice())?;
            println!(
                "Delegating oracle queue: {} with index: {}",
                queue, queue_struct.index
            );
            vec![delegate_oracle_queue(
                signer.pubkey(),
                queue,
                queue_struct.index,
            )]
        }
        Commands::UndelegateOracleQueue { queue } => {
            let queue = Pubkey::from_str(queue)?;
            let queue_account = match rpc_client.get_account(&queue) {
                Ok(acc) => acc,
                Err(e) => {
                    eprintln!("Error: {e}");
                    rpc_client.request_airdrop(&queue, 1)?;
                    exit(1);
                }
            };
            let queue_struct = Queue::try_from_bytes(queue_account.data.as_slice())?;
            println!(
                "Undelegating oracle queue: {} with index: {}",
                queue, queue_struct.index
            );
            vec![undelegate_oracle_queue(
                signer.pubkey(),
                queue,
                queue_struct.index,
            )]
        }
        Commands::CloseOracleQueue { queue } => {
            let queue = Pubkey::from_str(queue)?;
            let queue_account = rpc_client.get_account(&queue)?;
            let queue_struct = Queue::try_from_bytes(queue_account.data.as_slice())?;
            println!(
                "Closing oracle queue: {} with index: {}",
                queue, queue_struct.index
            );
            vec![close_oracle_queue(signer.pubkey(), queue_struct.index)]
        }
        Commands::DerivePubkey {} => {
            let (_, oracle_vrf_pk) = generate_vrf_keypair(&signer);
            let pk = Pubkey::from(oracle_vrf_pk.compress().to_bytes());
            println!("Derived pubkey for ({}): {}", signer.pubkey(), pk);
            exit(0)
        }
        Commands::ListQueue {} => {
            for i in 0..20 {
                let queue = oracle_queue_pda(&signer.pubkey(), i).0;
                let acc = rpc_client.get_account(&queue);
                if acc.is_ok() {
                    let account = acc?;
                    let queue_struct = Queue::try_from_bytes(account.data.as_slice())?;
                    println!(
                        "Queue address: {}, items: {}, index: {}, delegated: {}",
                        queue,
                        queue_struct.item_count,
                        queue_struct.index,
                        !account.owner.eq(&ephemeral_vrf_api::ID)
                    );
                }
            }
            exit(0)
        }
    };

    let mut ixs = Vec::with_capacity(1 + instructions.len());
    ixs.push(compute_budget_ix);
    ixs.extend_from_slice(&instructions);
    let transaction =
        Transaction::new_signed_with_payer(&ixs, Some(&signer.pubkey()), &[&signer], blockhash);

    let signature = rpc_client.send_and_confirm_transaction(&transaction)?;
    println!("Transaction signature: {signature}");

    Ok(())
}
