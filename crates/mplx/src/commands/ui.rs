//! Web UI command
//!
//! Launches the MPL web dashboard for monitoring and management.

use anyhow::{Context, Result};
use axum::response::Html;
use axum::routing::get;
use axum::Router;
use tokio::net::TcpListener;
use tracing::info;

/// Run the standalone UI server
pub async fn run(port: u16, open_browser: bool, data_dir: &str) -> Result<()> {
    let addr = format!("0.0.0.0:{}", port);
    let url = format!("http://localhost:{}", port);

    println!();
    println!("  MPL Dashboard");
    println!("  ─────────────────────────────────────");
    println!("  URL: {}", url);
    println!("  Data: {}", data_dir);
    println!();

    // Open browser if requested
    if open_browser {
        if let Err(e) = open::that(&url) {
            println!("  Could not open browser: {}", e);
            println!("  Please open {} manually", url);
        }
    }

    println!("  Press Ctrl+C to stop");
    println!();

    let data_dir = data_dir.to_string();
    let app = build_ui_app(&data_dir)?;

    let listener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("Failed to bind to {}", addr))?;

    info!("UI server listening on {}", addr);

    axum::serve(listener, app)
        .await
        .context("UI server error")?;

    Ok(())
}

/// Build the UI application
fn build_ui_app(data_dir: &str) -> Result<Router> {
    let data_dir = data_dir.to_string();

    let app = Router::new()
        // Main dashboard
        .route("/", get(serve_index))
        // API endpoints
        .route("/api/status", get(api_status))
        .route(
            "/api/schemas",
            get({
                let dd = data_dir.clone();
                move || api_schemas(dd.clone())
            }),
        )
        .route(
            "/api/traffic",
            get({
                let dd = data_dir.clone();
                move || api_traffic(dd.clone())
            }),
        )
        .route("/api/metrics", get(api_metrics))
        // QoM endpoints
        .route("/api/qom", get(api_qom_metrics))
        .route(
            "/api/qom/events",
            get({
                let dd = data_dir.clone();
                move |query| api_qom_events(dd.clone(), query)
            }),
        )
        .route(
            "/api/qom/history",
            get({
                let dd = data_dir.clone();
                move |query| api_qom_history(dd.clone(), query)
            }),
        )
        .route("/api/toc/pending", get(api_toc_pending))
        // Static assets (for Vue app)
        .route("/assets/*path", get(serve_assets));

    Ok(app)
}

/// Serve the main index page
async fn serve_index() -> Html<&'static str> {
    Html(include_str!("../ui/index.html"))
}

/// Serve static assets
async fn serve_assets(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> impl axum::response::IntoResponse {
    // For now, return 404 for assets - will be replaced with actual Vue build
    (
        axum::http::StatusCode::NOT_FOUND,
        format!("Asset not found: {}", path),
    )
}

/// API: Get system status
async fn api_status() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "running",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_seconds": 0,
        "proxy": {
            "connected": false,
            "upstream": null
        }
    }))
}

/// API: Get schemas
async fn api_schemas(data_dir: String) -> axum::Json<serde_json::Value> {
    let schemas_path = std::path::Path::new(&data_dir).join("schemas");

    // Try to load inference state
    let schemas = if let Ok(state) = super::schemas::InferenceState::load(&schemas_path) {
        state
            .schemas
            .into_iter()
            .map(|(stype, info)| {
                serde_json::json!({
                    "stype": stype,
                    "status": info.status,
                    "sample_count": info.sample_count,
                    "created_at": info.created_at,
                    "updated_at": info.updated_at
                })
            })
            .collect::<Vec<_>>()
    } else {
        vec![]
    };

    axum::Json(serde_json::json!({
        "schemas": schemas,
        "total": schemas.len()
    }))
}

/// API: Get traffic records
async fn api_traffic(data_dir: String) -> axum::Json<serde_json::Value> {
    let traffic_path = std::path::Path::new(&data_dir).join("traffic");

    let mut records = vec![];

    if traffic_path.exists() {
        if let Ok(entries) = std::fs::read_dir(&traffic_path) {
            for entry in entries.take(100).flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(record) = serde_json::from_str::<serde_json::Value>(&content) {
                            records.push(record);
                        }
                    }
                }
            }
        }
    }

    axum::Json(serde_json::json!({
        "records": records,
        "total": records.len()
    }))
}

/// API: Get metrics
async fn api_metrics() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "requests_total": 0,
        "requests_per_second": 0.0,
        "schema_pass_rate": 1.0,
        "qom_pass_rate": 1.0,
        "avg_latency_ms": 0.0,
        "active_connections": 0
    }))
}

/// API: Get QoM metrics summary
async fn api_qom_metrics() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "metrics": {
            "schema_fidelity": { "score": 1.0, "samples": 0, "failures": 0 },
            "instruction_compliance": { "score": null, "samples": 0, "failures": 0 },
            "tool_outcome_correctness": { "score": null, "samples": 0, "failures": 0, "pending": 0 },
            "groundedness": { "score": null, "samples": 0, "failures": 0 },
            "determinism_jitter": { "score": null, "samples": 0, "failures": 0 },
            "ontology_adherence": { "score": null, "samples": 0, "failures": 0 }
        }
    }))
}

/// Query parameters for events API
#[derive(serde::Deserialize, Default)]
struct EventsQuery {
    limit: Option<usize>,
}

/// API: Get QoM events
async fn api_qom_events(
    data_dir: String,
    axum::extract::Query(query): axum::extract::Query<EventsQuery>,
) -> axum::Json<serde_json::Value> {
    let limit = query.limit.unwrap_or(50);
    let events_path = std::path::Path::new(&data_dir).join("qom_events.json");

    let events = if events_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&events_path) {
            if let Ok(all_events) = serde_json::from_str::<Vec<serde_json::Value>>(&content) {
                all_events.into_iter().take(limit).collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    } else {
        // Return demo events for development
        vec![
            serde_json::json!({
                "id": "evt_001",
                "stype": "org.example.Rating.v1",
                "profile": "qom-basic",
                "passed": true,
                "scores": { "SF": 1.0 },
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
            serde_json::json!({
                "id": "evt_002",
                "stype": "org.example.Review.v1",
                "profile": "qom-strict-argcheck",
                "passed": false,
                "scores": { "SF": 1.0, "IC": 0.85 },
                "failure_reason": "IC score 0.85 below threshold 0.97",
                "timestamp": (chrono::Utc::now() - chrono::Duration::minutes(5)).to_rfc3339()
            }),
        ]
    };

    axum::Json(serde_json::json!({
        "events": events,
        "total": events.len()
    }))
}

/// Query parameters for history API
#[derive(serde::Deserialize, Default)]
struct HistoryQuery {
    period: Option<String>,
}

/// API: Get QoM history for trends
async fn api_qom_history(
    data_dir: String,
    axum::extract::Query(query): axum::extract::Query<HistoryQuery>,
) -> axum::Json<serde_json::Value> {
    let period = query.period.unwrap_or_else(|| "24h".to_string());
    let history_path = std::path::Path::new(&data_dir).join("qom_history.json");

    let history = if history_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&history_path) {
            serde_json::from_str::<Vec<serde_json::Value>>(&content).unwrap_or_default()
        } else {
            vec![]
        }
    } else {
        // Generate demo history based on period
        let now = chrono::Utc::now();
        let (duration, points) = match period.as_str() {
            "1h" => (chrono::Duration::hours(1), 12),
            "6h" => (chrono::Duration::hours(6), 12),
            "7d" => (chrono::Duration::days(7), 14),
            _ => (chrono::Duration::hours(24), 12), // 24h default
        };
        let interval = duration / points;

        (0..points)
            .map(|i| {
                let time = now - (duration - interval * i);
                serde_json::json!({
                    "timestamp": time.to_rfc3339(),
                    "sf": 0.95 + (i as f64 * 0.003).min(0.05),
                    "ic": 0.88 + (i as f64 * 0.005).min(0.12),
                    "toc": 0.85 + (i as f64 * 0.007).min(0.15),
                    "g": 0.80 + (i as f64 * 0.01).min(0.20),
                    "dj": 0.92 + (i as f64 * 0.004).min(0.08),
                    "oa": 0.90 + (i as f64 * 0.005).min(0.10)
                })
            })
            .collect()
    };

    axum::Json(serde_json::json!({
        "history": history,
        "period": period
    }))
}

/// API: Get pending TOC verifications
async fn api_toc_pending() -> axum::Json<serde_json::Value> {
    // In standalone UI mode, we don't have access to proxy state
    // This would be populated when connecting to a live proxy
    axum::Json(serde_json::json!({
        "verifications": [],
        "total": 0
    }))
}
