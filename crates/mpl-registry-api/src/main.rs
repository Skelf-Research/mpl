//! MPL Registry API Server

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use tokio::net::TcpListener;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use mpl_registry_api::{create_router, RegistryState};

#[derive(Parser, Debug)]
#[command(name = "mpl-registry-api")]
#[command(about = "MPL Registry API Server")]
#[command(version)]
struct Args {
    /// Listen address
    #[arg(short, long, default_value = "0.0.0.0:8081")]
    listen: String,

    /// Path to registry directory
    #[arg(short, long, default_value = "./registry")]
    registry: PathBuf,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Setup logging
    let level = if args.verbose { Level::DEBUG } else { Level::INFO };
    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .json()
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Create state
    let state = Arc::new(RegistryState::new(args.registry.clone()));

    // Create router
    let app = create_router(state);

    // Start server
    let addr: SocketAddr = args.listen.parse()?;
    let listener = TcpListener::bind(addr).await?;

    info!("MPL Registry API listening on {}", addr);
    info!("Registry path: {}", args.registry.display());

    axum::serve(listener, app).await?;

    Ok(())
}
