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
use serde_json::json;
use tracing::{debug, error, info};

use mpl_core::envelope::MplEnvelope;

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
        metrics.requests_total.load(std::sync::atomic::Ordering::Relaxed),
        metrics.schema_pass.load(std::sync::atomic::Ordering::Relaxed),
        metrics.schema_fail.load(std::sync::atomic::Ordering::Relaxed),
        schema_pass_rate,
        qom_pass_rate,
        metrics.handshakes.load(std::sync::atomic::Ordering::Relaxed),
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
    info!("AI-ALPN handshake from client with {} STypes", hello.stypes.len());

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
                        let validation = state.validate_request(&envelope);

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
                            serde_json::to_value(&select).unwrap_or_else(|_| json!({"error": "serialization failed"}))
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
                .unwrap()
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
        },
        "stypes": stypes,
        "profiles": profiles,
        "mode": format!("{:?}", state.config.mpl.mode),
    }))
}
