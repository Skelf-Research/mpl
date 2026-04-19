//! Zero-config proxy command
//!
//! Starts the MPL proxy with sensible defaults. Just point it at an upstream
//! and it starts working immediately.

use anyhow::{Context, Result};
use axum::Router;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, warn};

use mpl_proxy::config::{
    MplConfig, ObservabilityConfig, ProxyConfig, ProxyMode, TransportConfig,
};
use mpl_proxy::handlers;
use mpl_proxy::proxy::ProxyState;

use crate::Mode;

/// Run the zero-config proxy
#[allow(clippy::too_many_arguments)]
pub async fn run(
    upstream: &str,
    listen: &str,
    mode: Mode,
    learn: bool,
    schemas: Option<&str>,
    metrics_port: u16,
    ui_enabled: bool,
    ui_port: u16,
    data_dir: &str,
    verbose: bool,
) -> Result<()> {
    // Ensure data directory exists
    let data_path = std::path::Path::new(data_dir);
    if !data_path.exists() {
        std::fs::create_dir_all(data_path)
            .with_context(|| format!("Failed to create data directory: {}", data_dir))?;
    }

    // Normalize upstream URL
    let upstream_url = normalize_upstream(upstream);

    // Map CLI mode to proxy mode
    let proxy_mode = match mode {
        Mode::Development => ProxyMode::Transparent,
        Mode::Production => ProxyMode::Strict,
    };

    // Determine registry/schemas path
    let registry_path = schemas
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{}/schemas", data_dir));

    // Build configuration with sensible defaults
    let config = ProxyConfig {
        transport: TransportConfig {
            listen: listen.to_string(),
            upstream: upstream_url.clone(),
            ..Default::default()
        },
        mpl: MplConfig {
            registry: registry_path.clone(),
            mode: proxy_mode,
            required_profile: Some(match mode {
                Mode::Development => "qom-basic".to_string(),
                Mode::Production => "qom-strict-argcheck".to_string(),
            }),
            enforce_schema: matches!(mode, Mode::Production),
            enforce_assertions: matches!(mode, Mode::Production),
            ..Default::default()
        },
        observability: ObservabilityConfig {
            metrics_port: if metrics_port > 0 {
                Some(metrics_port)
            } else {
                None
            },
            ..Default::default()
        },
        routing: vec![],
        limits: Default::default(),
    };

    // Create proxy state
    let state = Arc::new(
        ProxyState::new(config.clone())
            .await
            .context("Failed to initialize proxy state")?,
    );

    // Build the main application
    let app = build_app(state.clone(), learn, data_dir)?;

    // Print startup banner
    print_banner(&upstream_url, listen, mode, learn, metrics_port, ui_enabled, ui_port);

    // Start metrics server if enabled
    if metrics_port > 0 {
        let metrics_state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = start_metrics_server(metrics_port, metrics_state).await {
                warn!("Metrics server failed: {}", e);
            }
        });
    }

    // Start UI server if enabled
    if ui_enabled {
        let ui_data_dir = data_dir.to_string();
        tokio::spawn(async move {
            if let Err(e) = start_ui_server(ui_port, &ui_data_dir).await {
                warn!("UI server failed: {}", e);
            }
        });
    }

    // Start the main proxy server
    let listener = TcpListener::bind(listen)
        .await
        .with_context(|| format!("Failed to bind to {}", listen))?;

    info!("Proxy listening on {}", listen);

    axum::serve(listener, app)
        .await
        .context("Proxy server error")?;

    Ok(())
}

/// Build the main Axum application
fn build_app(
    state: Arc<ProxyState>,
    _learn: bool,
    _data_dir: &str,
) -> Result<Router> {
    let app = Router::new()
        .route("/health", axum::routing::get(handlers::health))
        .route("/capabilities", axum::routing::get(handlers::capabilities))
        .route(
            "/.well-known/ai-alpn",
            axum::routing::post(handlers::ai_alpn_handshake),
        )
        .route("/*path", axum::routing::any(handlers::proxy_handler))
        .route("/", axum::routing::any(handlers::proxy_handler))
        .with_state(state);

    Ok(app)
}

/// Start the metrics server
async fn start_metrics_server(port: u16, _state: Arc<ProxyState>) -> Result<()> {
    use axum::routing::get;

    let app = Router::new().route(
        "/metrics",
        get(|| async {
            // Basic prometheus metrics endpoint
            // TODO: Integrate with ProxyState metrics
            "# MPL Proxy Metrics\nmpl_requests_total 0\n"
        }),
    );

    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    info!("Metrics server listening on {}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}

/// Start the UI server
async fn start_ui_server(port: u16, _data_dir: &str) -> Result<()> {
    use axum::routing::get;
    use axum::response::Html;

    // Basic UI placeholder - will be replaced with Vue app
    let app = Router::new()
        .route("/", get(|| async {
            Html(include_str!("../ui/index.html"))
        }))
        .route("/api/status", get(|| async {
            axum::Json(serde_json::json!({
                "status": "running",
                "version": env!("CARGO_PKG_VERSION")
            }))
        }));

    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    info!("UI server listening on http://{}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}

/// Normalize upstream URL to include protocol
fn normalize_upstream(upstream: &str) -> String {
    if upstream.starts_with("http://") || upstream.starts_with("https://") {
        // Strip protocol for internal use (just host:port)
        upstream
            .strip_prefix("http://")
            .or_else(|| upstream.strip_prefix("https://"))
            .unwrap_or(upstream)
            .to_string()
    } else {
        upstream.to_string()
    }
}

/// Print startup banner
fn print_banner(
    upstream: &str,
    listen: &str,
    mode: Mode,
    learn: bool,
    metrics_port: u16,
    ui_enabled: bool,
    ui_port: u16,
) {
    println!();
    println!("  MPL Proxy v{}", env!("CARGO_PKG_VERSION"));
    println!("  ─────────────────────────────────────");
    println!("  Upstream:   http://{}", upstream);
    println!("  Listen:     {}", listen);
    println!("  Mode:       {:?}", mode);
    println!("  Learning:   {}", if learn { "enabled" } else { "disabled" });
    if metrics_port > 0 {
        println!("  Metrics:    http://0.0.0.0:{}/metrics", metrics_port);
    }
    if ui_enabled {
        println!("  Dashboard:  http://0.0.0.0:{}", ui_port);
    }
    println!();
    println!("  Press Ctrl+C to stop");
    println!();
}
