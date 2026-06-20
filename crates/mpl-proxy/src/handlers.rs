//! HTTP handlers for the proxy

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::{Request, Response, StatusCode},
    response::IntoResponse,
    Json,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, error, info};

use mpl_core::envelope::MplEnvelope;
use mpl_core::metrics::{TocMethod, TocResult};

use crate::proxy::{AiAlpnClientHello, ProxyState};

/// Health check endpoint
pub async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// Prometheus metrics endpoint
pub async fn metrics(State(state): State<Arc<ProxyState>>) -> impl IntoResponse {
    let metrics = &state.metrics;
    let schema_pass_rate = metrics.schema_pass_rate();
    let qom_pass_rate = metrics.qom_pass_rate();
    let downgrade_rate = metrics.downgrade_rate();

    let output = format!(
        r#"# HELP mpl_requests_total Total number of requests
# TYPE mpl_requests_total counter
mpl_requests_total {}

# HELP mpl_schema_validations_total Schema validation results
# TYPE mpl_schema_validations_total counter
mpl_schema_validations_total{{result="pass"}} {}
mpl_schema_validations_total{{result="fail"}} {}

# HELP mpl_schema_pass_rate Schema validation pass rate
# TYPE mpl_schema_pass_rate gauge
mpl_schema_pass_rate {}

# HELP mpl_qom_pass_rate QoM pass rate
# TYPE mpl_qom_pass_rate gauge
mpl_qom_pass_rate {}

# HELP mpl_handshakes_total Total AI-ALPN handshakes
# TYPE mpl_handshakes_total counter
mpl_handshakes_total {}

# HELP mpl_downgrade_rate Protocol downgrade rate
# TYPE mpl_downgrade_rate gauge
mpl_downgrade_rate {}
"#,
        metrics
            .requests_total
            .load(std::sync::atomic::Ordering::Relaxed),
        metrics
            .schema_pass
            .load(std::sync::atomic::Ordering::Relaxed),
        metrics
            .schema_fail
            .load(std::sync::atomic::Ordering::Relaxed),
        schema_pass_rate,
        qom_pass_rate,
        metrics
            .handshakes
            .load(std::sync::atomic::Ordering::Relaxed),
        downgrade_rate,
    );

    (
        StatusCode::OK,
        [("content-type", "text/plain; charset=utf-8")],
        output,
    )
}

/// AI-ALPN handshake endpoint
pub async fn ai_alpn_handshake(
    State(state): State<Arc<ProxyState>>,
    Json(hello): Json<AiAlpnClientHello>,
) -> impl IntoResponse {
    info!(
        "AI-ALPN handshake from client with {} STypes",
        hello.stypes.len()
    );

    let response = state.handle_handshake(hello);

    info!(
        "Negotiated {} common STypes, profile: {:?}",
        response.common_stypes.len(),
        response.selected_profile
    );

    Json(response)
}

/// WebSocket upgrade handler for MCP/A2A connections
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ProxyState>>,
) -> impl IntoResponse {
    info!("WebSocket upgrade requested");
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

/// Handle WebSocket connection
async fn handle_websocket(socket: WebSocket, state: Arc<ProxyState>) {
    let (mut sender, mut receiver) = socket.split();

    info!("WebSocket connection established");

    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                debug!("Received WebSocket message: {} bytes", text.len());

                // Try to parse as MPL envelope
                let response = match serde_json::from_str::<MplEnvelope>(&text) {
                    Ok(envelope) => {
                        // Validate the envelope
                        let validation = state.validate_request(&envelope).await;

                        if !validation.valid && state.is_strict() {
                            // Return error response
                            json!({
                                "error": "E-SCHEMA-FIDELITY",
                                "message": "Validation failed",
                                "details": validation.errors,
                            })
                        } else {
                            // Forward to upstream (simplified - in real impl, maintain upstream WS connection)
                            // For now, echo back with validation result
                            json!({
                                "type": "mpl-response",
                                "stype": envelope.stype,
                                "validation": {
                                    "valid": validation.valid,
                                    "schema_valid": validation.schema_valid,
                                    "qom_passed": validation.qom_passed,
                                },
                                "payload": envelope.payload,
                            })
                        }
                    }
                    Err(_) => {
                        // Try to parse as AI-ALPN handshake
                        if let Ok(hello) = serde_json::from_str::<AiAlpnClientHello>(&text) {
                            let select = state.handle_handshake(hello);
                            serde_json::to_value(&select)
                                .unwrap_or_else(|_| json!({"error": "serialization failed"}))
                        } else {
                            // Pass through non-MPL messages
                            json!({
                                "type": "passthrough",
                                "message": text,
                            })
                        }
                    }
                };

                if let Err(e) = sender.send(Message::Text(response.to_string())).await {
                    error!("Failed to send WebSocket message: {}", e);
                    break;
                }
            }
            Ok(Message::Binary(data)) => {
                debug!("Received binary WebSocket message: {} bytes", data.len());
                // Pass through binary messages
                if let Err(e) = sender.send(Message::Binary(data)).await {
                    error!("Failed to send binary WebSocket message: {}", e);
                    break;
                }
            }
            Ok(Message::Ping(data)) => {
                if let Err(e) = sender.send(Message::Pong(data)).await {
                    error!("Failed to send pong: {}", e);
                    break;
                }
            }
            Ok(Message::Pong(_)) => {}
            Ok(Message::Close(_)) => {
                info!("WebSocket connection closed by client");
                break;
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
        }
    }

    info!("WebSocket connection ended");
}

/// Main proxy handler - forwards requests to upstream
pub async fn proxy_handler(
    State(state): State<Arc<ProxyState>>,
    Path(path): Path<String>,
    request: Request<Body>,
) -> impl IntoResponse {
    debug!("Proxying request to: {}", path);

    match state.forward_request(path, request).await {
        Ok(response) => response,
        Err(e) => {
            error!("Proxy error: {}", e);
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "error": "E-PROXY-ERROR",
                        "message": format!("Proxy error: {}", e),
                    })
                    .to_string(),
                ))
                .expect("static BAD_GATEWAY response with literal header + JSON body always builds")
        }
    }
}

/// Server capabilities endpoint
pub async fn capabilities(State(state): State<Arc<ProxyState>>) -> impl IntoResponse {
    let stypes = state.validator.registered_stypes();
    let profiles: Vec<&str> = state.profiles.iter().map(|p| p.name.as_str()).collect();

    Json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "mpl_version": "1.0",
        "capabilities": {
            "schema_validation": state.config.mpl.enforce_schema,
            "qom_evaluation": true,
            "semantic_hashing": true,
            "websocket": true,
            "toc_callback": true,
        },
        "stypes": stypes,
        "profiles": profiles,
        "mode": format!("{:?}", state.config.mpl.mode),
    }))
}

/// TOC callback request body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocCallbackRequest {
    /// The callback ID from the original request
    pub callback_id: String,
    /// Whether the tool outcome was verified
    pub verified: bool,
    /// Optional details about the verification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    /// Expected outcome (for audit)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    /// Actual outcome observed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
}

/// TOC callback response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocCallbackResponse {
    /// Whether the callback was accepted
    pub accepted: bool,
    /// The callback ID
    pub callback_id: String,
    /// Status message
    pub message: String,
}

/// TOC callback endpoint - receives verification results from external systems
///
/// POST /_mpl/toc/callback
/// Body: { "callback_id": "...", "verified": true/false, "details": "..." }
pub async fn toc_callback(
    State(state): State<Arc<ProxyState>>,
    Json(request): Json<TocCallbackRequest>,
) -> impl IntoResponse {
    info!(
        "TOC callback received: {} verified={}",
        request.callback_id, request.verified
    );

    // Build the TOC result
    let result = if request.verified {
        let mut r = TocResult::verified(TocMethod::Callback);
        r.details = request.details.clone();
        r.expected = request.expected;
        r.actual = request.actual;
        r
    } else {
        let mut r = TocResult::failed(
            TocMethod::Callback,
            request
                .details
                .clone()
                .unwrap_or_else(|| "Verification failed".to_string()),
        );
        r.expected = request.expected;
        r.actual = request.actual;
        r
    };

    // Complete the verification
    let was_pending = state.complete_toc(&request.callback_id, result);

    let response = if was_pending {
        TocCallbackResponse {
            accepted: true,
            callback_id: request.callback_id,
            message: "TOC verification recorded".to_string(),
        }
    } else {
        TocCallbackResponse {
            accepted: false,
            callback_id: request.callback_id,
            message: "Unknown or expired callback ID".to_string(),
        }
    };

    Json(response)
}

/// Query TOC status for a callback ID
///
/// GET /_mpl/toc/status/{callback_id}
pub async fn toc_status(
    State(state): State<Arc<ProxyState>>,
    Path(callback_id): Path<String>,
) -> impl IntoResponse {
    // Check if completed
    if let Some(result) = state.get_toc_result(&callback_id) {
        return Json(json!({
            "callback_id": callback_id,
            "status": "completed",
            "result": result,
        }));
    }

    // Check if pending
    if let Some(pending) = state.get_pending_toc(&callback_id) {
        return Json(json!({
            "callback_id": callback_id,
            "status": "pending",
            "stype": pending.stype,
            "registered_at": pending.timestamp,
        }));
    }

    // Unknown
    Json(json!({
        "callback_id": callback_id,
        "status": "unknown",
        "message": "No verification found for this callback ID",
    }))
}

/// List all pending TOC verifications
///
/// GET /_mpl/toc/pending
pub async fn toc_pending_list(State(state): State<Arc<ProxyState>>) -> impl IntoResponse {
    let pending: Vec<_> = state
        .pending_toc
        .read()
        .map(|p| p.values().cloned().collect())
        .unwrap_or_default();

    Json(json!({
        "pending_count": pending.len(),
        "verifications": pending,
    }))
}

// ============ QoM API Endpoints ============

/// Query parameters for QoM events
#[derive(Debug, Deserialize, Default)]
pub struct QomEventsQuery {
    /// Maximum number of events to return
    pub limit: Option<usize>,
}

/// Query parameters for QoM history
#[derive(Debug, Deserialize, Default)]
pub struct QomHistoryQuery {
    /// Time period: "1h", "6h", "24h", "7d"
    pub period: Option<String>,
}

/// Get QoM metrics summary
///
/// GET /_mpl/qom
pub async fn qom_summary(State(state): State<Arc<ProxyState>>) -> impl IntoResponse {
    let summary = state.qom_recorder.get_summary().await;

    Json(json!({
        "metrics": {
            "schema_fidelity": summary.schema_fidelity,
            "instruction_compliance": summary.instruction_compliance,
            "tool_outcome_correctness": summary.tool_outcome_correctness,
            "groundedness": summary.groundedness,
            "determinism_jitter": summary.determinism_jitter,
            "ontology_adherence": summary.ontology_adherence,
        }
    }))
}

/// Get recent QoM events
///
/// GET /_mpl/qom/events?limit=50
pub async fn qom_events(
    State(state): State<Arc<ProxyState>>,
    axum::extract::Query(query): axum::extract::Query<QomEventsQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(50);
    let events = state.qom_recorder.get_events(limit).await;

    // Convert events to JSON-friendly format
    let events_json: Vec<serde_json::Value> = events
        .iter()
        .map(|e| {
            json!({
                "id": e.id,
                "stype": e.stype,
                "profile": e.profile,
                "passed": e.passed,
                "scores": {
                    "SF": e.scores.sf,
                    "IC": e.scores.ic,
                    "TOC": e.scores.toc,
                    "G": e.scores.g,
                    "DJ": e.scores.dj,
                    "OA": e.scores.oa,
                },
                "failure_reason": e.failure_reason,
                "timestamp": e.timestamp.to_rfc3339(),
            })
        })
        .collect();

    Json(json!({
        "events": events_json,
        "total": events_json.len(),
    }))
}

/// Get QoM history for trends
///
/// GET /_mpl/qom/history?period=24h
pub async fn qom_history(
    State(state): State<Arc<ProxyState>>,
    axum::extract::Query(query): axum::extract::Query<QomHistoryQuery>,
) -> impl IntoResponse {
    let period = query.period.unwrap_or_else(|| "24h".to_string());
    let history = state.qom_recorder.get_history(&period).await;

    // Convert history to JSON-friendly format
    let history_json: Vec<serde_json::Value> = history
        .iter()
        .map(|h| {
            json!({
                "timestamp": h.timestamp.to_rfc3339(),
                "count": h.count,
                "sf": h.sf,
                "ic": h.ic,
                "toc": h.toc,
                "g": h.g,
                "dj": h.dj,
                "oa": h.oa,
                "pass_rate": h.pass_rate,
            })
        })
        .collect();

    Json(json!({
        "history": history_json,
        "period": period,
    }))
}

/// Persist QoM history to disk (for maintenance)
///
/// POST /_mpl/qom/persist
pub async fn qom_persist(State(state): State<Arc<ProxyState>>) -> impl IntoResponse {
    state.qom_recorder.persist_history().await;

    Json(json!({
        "status": "ok",
        "message": "QoM history persisted to disk",
    }))
}

/// Get learning/traffic recording statistics
///
/// GET /_mpl/learning/stats
pub async fn learning_stats(State(state): State<Arc<ProxyState>>) -> impl IntoResponse {
    let enabled = state.traffic_recorder.is_enabled();
    let stats = state.traffic_recorder.get_stats();

    let total_samples: usize = stats.values().sum();
    let stype_count = stats.len();

    // Get top stypes by sample count
    let mut stypes_sorted: Vec<_> = stats.into_iter().collect();
    stypes_sorted.sort_by(|a, b| b.1.cmp(&a.1));
    let top_stypes: Vec<_> = stypes_sorted.into_iter().take(20).collect();

    Json(json!({
        "enabled": enabled,
        "total_samples": total_samples,
        "stype_count": stype_count,
        "top_stypes": top_stypes.iter().map(|(stype, count)| {
            json!({
                "stype": stype,
                "samples": count
            })
        }).collect::<Vec<_>>()
    }))
}

/// Get traffic samples for a specific SType
///
/// GET /_mpl/learning/samples/:stype
pub async fn learning_samples(
    State(state): State<Arc<ProxyState>>,
    axum::extract::Path(stype): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<LearningQuery>,
) -> impl IntoResponse {
    let samples = state.traffic_recorder.get_samples(&stype);
    let limit = query.limit.unwrap_or(50);

    let samples_json: Vec<serde_json::Value> = samples
        .iter()
        .rev()
        .take(limit)
        .map(|s| {
            json!({
                "id": s.id,
                "timestamp": s.timestamp,
                "method": s.method,
                "path": s.path,
                "payload": s.payload,
                "response": s.response,
                "status_code": s.status_code,
                "duration_ms": s.duration_ms,
                "validation_passed": s.validation_passed,
            })
        })
        .collect();

    Json(json!({
        "stype": stype,
        "samples": samples_json,
        "total": samples.len(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct LearningQuery {
    pub limit: Option<usize>,
}
