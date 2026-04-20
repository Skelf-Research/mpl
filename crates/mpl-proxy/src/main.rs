//! MPL Sidecar Proxy
//!
//! Intercepts MCP/A2A traffic and adds MPL handshake, schema validation,
//! QoM checks, and provenance logging.

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::{routing::{any, get, post}, Router};
use clap::Parser;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use mpl_proxy::config::ProxyConfig;
use mpl_proxy::middleware::MplMiddleware;
use mpl_proxy::proxy::ProxyState;
use mpl_proxy::handlers;

/// MPL Sidecar Proxy
#[derive(Parser, Debug)]
#[command(name = "mpl-proxy")]
#[command(about = "MPL sidecar proxy for MCP/A2A traffic")]
#[command(version)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "mpl-config.yaml")]
    config: String,

    /// Listen address
    #[arg(short, long)]
    listen: Option<String>,

    /// Upstream server address
    #[arg(short, long)]
    upstream: Option<String>,

    /// Path to local registry
    #[arg(short, long)]
    registry: Option<String>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Set up logging
    let level = if args.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };
    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .json()
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Load configuration with environment variable overrides
    let mut config = ProxyConfig::load_with_env(&args.config).unwrap_or_else(|e| {
        info!("Could not load config file: {}, using defaults with env vars", e);
        let mut cfg = ProxyConfig::default();
        cfg.apply_env_overrides();
        cfg
    });

    // Override with CLI args (highest priority)
    if let Some(listen) = args.listen {
        config.transport.listen = listen;
    }
    if let Some(upstream) = args.upstream {
        config.transport.upstream = upstream;
    }
    if let Some(registry) = args.registry {
        config.mpl.registry = registry;
    }

    info!("Starting MPL proxy");
    info!("Listen: {}", config.transport.listen);
    info!("Upstream: {}", config.transport.upstream);
    info!("Registry: {}", config.mpl.registry);
    info!("Profile: {:?}", config.mpl.required_profile);
    info!("Mode: {:?}", config.mpl.mode);

    // Create shared state
    let proxy_state = Arc::new(ProxyState::new(config.clone()).await?);

    // Build router with all MPL endpoints
    let app = Router::new()
        // Health and capabilities
        .route("/health", get(handlers::health))
        .route("/capabilities", get(handlers::capabilities))
        // AI-ALPN handshake
        .route("/.well-known/ai-alpn", post(handlers::ai_alpn_handshake))
        // Metrics
        .route("/metrics", get(handlers::metrics))
        // WebSocket endpoint
        .route("/ws", get(handlers::websocket_handler))
        // TOC callback endpoints (for Tool Outcome Correctness verification)
        .route("/_mpl/toc/callback", post(handlers::toc_callback))
        .route("/_mpl/toc/status/:callback_id", get(handlers::toc_status))
        .route("/_mpl/toc/pending", get(handlers::toc_pending_list))
        // QoM endpoints (for Quality of Meaning metrics)
        .route("/_mpl/qom", get(handlers::qom_summary))
        .route("/_mpl/qom/events", get(handlers::qom_events))
        .route("/_mpl/qom/history", get(handlers::qom_history))
        .route("/_mpl/qom/persist", post(handlers::qom_persist))
        // Learning endpoints (for traffic recording and schema inference)
        .route("/_mpl/learning/stats", get(handlers::learning_stats))
        .route("/_mpl/learning/samples/:stype", get(handlers::learning_samples))
        // Proxy all other requests
        .route("/*path", any(handlers::proxy_handler))
        .with_state(proxy_state.clone())
        .layer(ServiceBuilder::new().layer(MplMiddleware::new(proxy_state.clone())));

    // Start main proxy server
    let addr: SocketAddr = config.transport.listen.parse()?;
    let listener = TcpListener::bind(addr).await?;
    info!("Proxy server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
