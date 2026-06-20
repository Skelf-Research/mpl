//! MPL Validation Integration Tests
//!
//! Tests the MPL proxy validation pipeline: schema validation, QoM evaluation,
//! semantic hashing, and error responses.

use axum::{routing::post, Json, Router};
use mpl_core::envelope::MplEnvelope;
use mpl_proxy::config::{MplConfig, ObservabilityConfig, ProxyConfig, ProxyMode, TransportConfig};
use mpl_proxy::handlers;
use mpl_proxy::proxy::ProxyState;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::sleep;

/// Registry path relative to the crate directory (tests run from crates/mpl-proxy)
const REGISTRY_PATH: &str = "../../registry";

/// Start a mock upstream server that echoes requests
async fn start_mock_upstream() -> std::net::SocketAddr {
    let app = Router::new()
        .route(
            "/",
            post(|Json(body): Json<Value>| async move { Json(body) }),
        )
        .route(
            "/*path",
            post(|Json(body): Json<Value>| async move { Json(body) }),
        );

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    sleep(Duration::from_millis(50)).await;
    addr
}

/// Start MPL proxy with specified mode
async fn start_mpl_proxy(
    upstream_addr: std::net::SocketAddr,
    mode: ProxyMode,
    registry_path: &str,
) -> std::net::SocketAddr {
    let config = ProxyConfig {
        transport: TransportConfig {
            listen: "127.0.0.1:0".to_string(),
            upstream: upstream_addr.to_string(),
            ..Default::default()
        },
        mpl: MplConfig {
            registry: registry_path.to_string(),
            mode,
            required_profile: Some("qom-basic".to_string()),
            enforce_schema: true,
            ..Default::default()
        },
        observability: ObservabilityConfig {
            metrics_port: None,
            ..Default::default()
        },
        routing: vec![],
        limits: Default::default(),
    };

    let state = Arc::new(ProxyState::new(config).await.unwrap());

    let app = Router::new()
        .route("/health", axum::routing::get(handlers::health))
        .route("/capabilities", axum::routing::get(handlers::capabilities))
        .route(
            "/.well-known/ai-alpn",
            axum::routing::post(handlers::ai_alpn_handshake),
        )
        .route("/metrics", axum::routing::get(handlers::metrics))
        .route("/*path", axum::routing::any(handlers::proxy_handler))
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    sleep(Duration::from_millis(50)).await;
    addr
}

// =============================================================================
// Schema Validation Tests
// =============================================================================

#[tokio::test]
async fn test_valid_envelope_passes_through() {
    let upstream = start_mock_upstream().await;
    let proxy = start_mpl_proxy(upstream, ProxyMode::Strict, REGISTRY_PATH).await;

    let client = reqwest::Client::new();

    // Create a valid calendar event envelope
    let mut envelope = MplEnvelope::new(
        "org.calendar.Event.v1".to_string(),
        json!({
            "title": "Team Meeting",
            "start": "2024-01-15T10:00:00Z",
            "end": "2024-01-15T11:00:00Z"
        }),
    );
    envelope.compute_hash().unwrap();

    let response = client
        .post(format!("http://{}/mcp", proxy))
        .json(&envelope)
        .send()
        .await
        .unwrap();

    let status = response.status();
    let qom_result = response
        .headers()
        .get("X-MPL-QoM-Result")
        .map(|v| v.to_str().unwrap().to_string());
    let body: Value = response.json().await.unwrap();

    assert!(
        status.is_success(),
        "Valid envelope should pass through. Status: {}, Body: {}",
        status,
        serde_json::to_string_pretty(&body).unwrap()
    );

    // Check QoM result header
    assert!(qom_result.is_some(), "QoM result header should be present");
    assert_eq!(qom_result.unwrap(), "pass");
}

#[tokio::test]
async fn test_schema_validation_failure_in_strict_mode() {
    let upstream = start_mock_upstream().await;
    let proxy = start_mpl_proxy(upstream, ProxyMode::Strict, REGISTRY_PATH).await;

    let client = reqwest::Client::new();

    // Create an invalid calendar event (missing required fields)
    let envelope = MplEnvelope::new(
        "org.calendar.Event.v1".to_string(),
        json!({
            "title": "Team Meeting"
            // Missing: start, end (required fields)
        }),
    );

    let response = client
        .post(format!("http://{}/mcp", proxy))
        .json(&envelope)
        .send()
        .await
        .unwrap();

    // Should return 400 Bad Request in strict mode
    assert_eq!(
        response.status(),
        reqwest::StatusCode::BAD_REQUEST,
        "Schema validation failure should return 400 in strict mode"
    );

    let body: Value = response.json().await.unwrap();
    assert_eq!(body["error"], "E-SCHEMA-FIDELITY");
    assert!(!body["details"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_schema_validation_failure_in_transparent_mode() {
    let upstream = start_mock_upstream().await;
    let proxy = start_mpl_proxy(upstream, ProxyMode::Transparent, REGISTRY_PATH).await;

    let client = reqwest::Client::new();

    // Create an invalid calendar event (missing required fields)
    let envelope = MplEnvelope::new(
        "org.calendar.Event.v1".to_string(),
        json!({
            "title": "Team Meeting"
            // Missing: start, end (required fields)
        }),
    );

    let response = client
        .post(format!("http://{}/mcp", proxy))
        .json(&envelope)
        .send()
        .await
        .unwrap();

    let qom_result = response
        .headers()
        .get("X-MPL-QoM-Result")
        .map(|v| v.to_str().unwrap().to_string());

    // Should pass through in transparent mode (with QoM fail header)
    assert!(
        response.status().is_success(),
        "Invalid envelope should pass through in transparent mode"
    );

    // QoM header should indicate failure
    assert!(qom_result.is_some());
    assert_eq!(qom_result.unwrap(), "fail");
}

#[tokio::test]
async fn test_unknown_stype_in_strict_mode() {
    let upstream = start_mock_upstream().await;
    let proxy = start_mpl_proxy(upstream, ProxyMode::Strict, REGISTRY_PATH).await;

    let client = reqwest::Client::new();

    // Create envelope with unknown SType
    let envelope = MplEnvelope::new(
        "org.unknown.Mystery.v1".to_string(),
        json!({
            "data": "some data"
        }),
    );

    let response = client
        .post(format!("http://{}/mcp", proxy))
        .json(&envelope)
        .send()
        .await
        .unwrap();

    // Should return 400 for unknown SType in strict mode
    assert_eq!(
        response.status(),
        reqwest::StatusCode::BAD_REQUEST,
        "Unknown SType should return 400 in strict mode"
    );

    let body: Value = response.json().await.unwrap();
    assert_eq!(body["error"], "E-SCHEMA-FIDELITY");
}

#[tokio::test]
async fn test_unknown_stype_in_transparent_mode() {
    let upstream = start_mock_upstream().await;
    let proxy = start_mpl_proxy(upstream, ProxyMode::Transparent, REGISTRY_PATH).await;

    let client = reqwest::Client::new();

    // Create envelope with unknown SType
    let envelope = MplEnvelope::new(
        "org.unknown.Mystery.v1".to_string(),
        json!({
            "data": "some data"
        }),
    );

    let response = client
        .post(format!("http://{}/mcp", proxy))
        .json(&envelope)
        .send()
        .await
        .unwrap();

    // Should pass through in transparent mode
    assert!(
        response.status().is_success(),
        "Unknown SType should pass through in transparent mode"
    );
}

// =============================================================================
// Semantic Hash Tests
// =============================================================================

#[tokio::test]
async fn test_valid_semantic_hash() {
    let upstream = start_mock_upstream().await;
    let proxy = start_mpl_proxy(upstream, ProxyMode::Strict, REGISTRY_PATH).await;

    let client = reqwest::Client::new();

    // Create envelope with valid hash
    let mut envelope = MplEnvelope::new(
        "org.calendar.Event.v1".to_string(),
        json!({
            "title": "Team Meeting",
            "start": "2024-01-15T10:00:00Z",
            "end": "2024-01-15T11:00:00Z"
        }),
    );
    envelope.compute_hash().unwrap();

    let response = client
        .post(format!("http://{}/mcp", proxy))
        .json(&envelope)
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success());
}

#[tokio::test]
async fn test_invalid_semantic_hash_in_strict_mode() {
    let upstream = start_mock_upstream().await;
    let proxy = start_mpl_proxy(upstream, ProxyMode::Strict, REGISTRY_PATH).await;

    let client = reqwest::Client::new();

    // Create envelope with incorrect hash
    let mut envelope = MplEnvelope::new(
        "org.calendar.Event.v1".to_string(),
        json!({
            "title": "Team Meeting",
            "start": "2024-01-15T10:00:00Z",
            "end": "2024-01-15T11:00:00Z"
        }),
    );
    // Set a fake hash
    envelope.sem_hash =
        Some("b3:0000000000000000000000000000000000000000000000000000000000000000".to_string());

    let response = client
        .post(format!("http://{}/mcp", proxy))
        .json(&envelope)
        .send()
        .await
        .unwrap();

    // Should return 400 for hash mismatch
    assert_eq!(
        response.status(),
        reqwest::StatusCode::BAD_REQUEST,
        "Hash mismatch should return 400 in strict mode"
    );

    let body: Value = response.json().await.unwrap();
    assert!(body["details"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e.as_str().unwrap().contains("hash")));
}

// =============================================================================
// Header-Based SType Tests
// =============================================================================

#[tokio::test]
async fn test_stype_from_header() {
    let upstream = start_mock_upstream().await;
    let proxy = start_mpl_proxy(upstream, ProxyMode::Strict, REGISTRY_PATH).await;

    let client = reqwest::Client::new();

    // Send plain JSON with SType header
    let response = client
        .post(format!("http://{}/mcp", proxy))
        .header("X-MPL-SType", "org.calendar.Event.v1")
        .json(&json!({
            "title": "Team Meeting",
            "start": "2024-01-15T10:00:00Z",
            "end": "2024-01-15T11:00:00Z"
        }))
        .send()
        .await
        .unwrap();

    assert!(
        response.status().is_success(),
        "Request with SType header should be validated"
    );
}

#[tokio::test]
async fn test_invalid_payload_with_stype_header_strict() {
    let upstream = start_mock_upstream().await;
    let proxy = start_mpl_proxy(upstream, ProxyMode::Strict, REGISTRY_PATH).await;

    let client = reqwest::Client::new();

    // Send invalid JSON with SType header
    let response = client
        .post(format!("http://{}/mcp", proxy))
        .header("X-MPL-SType", "org.calendar.Event.v1")
        .json(&json!({
            "title": "Team Meeting"
            // Missing required fields
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        reqwest::StatusCode::BAD_REQUEST,
        "Invalid payload with SType header should fail in strict mode"
    );
}

// =============================================================================
// Metrics Tests
// =============================================================================

#[tokio::test]
async fn test_metrics_updated_on_validation() {
    let upstream = start_mock_upstream().await;
    let proxy = start_mpl_proxy(upstream, ProxyMode::Transparent, REGISTRY_PATH).await;

    let client = reqwest::Client::new();

    // Send a valid request
    let mut valid_envelope = MplEnvelope::new(
        "org.calendar.Event.v1".to_string(),
        json!({
            "title": "Valid Event",
            "start": "2024-01-15T10:00:00Z",
            "end": "2024-01-15T11:00:00Z"
        }),
    );
    valid_envelope.compute_hash().unwrap();

    client
        .post(format!("http://{}/mcp", proxy))
        .json(&valid_envelope)
        .send()
        .await
        .unwrap();

    // Send an invalid request
    let invalid_envelope = MplEnvelope::new(
        "org.calendar.Event.v1".to_string(),
        json!({
            "title": "Invalid Event"
            // Missing required fields
        }),
    );

    client
        .post(format!("http://{}/mcp", proxy))
        .json(&invalid_envelope)
        .send()
        .await
        .unwrap();

    // Check metrics
    let metrics_response = client
        .get(format!("http://{}/metrics", proxy))
        .send()
        .await
        .unwrap();

    let metrics_text = metrics_response.text().await.unwrap();

    // Should have at least 2 requests
    assert!(metrics_text.contains("mpl_requests_total"));
    assert!(metrics_text.contains("mpl_schema_validations_total"));
    assert!(metrics_text.contains("mpl_schema_pass_rate"));
}

// =============================================================================
// Capabilities & Handshake Tests
// =============================================================================

#[tokio::test]
async fn test_capabilities_lists_stypes() {
    let upstream = start_mock_upstream().await;
    let proxy = start_mpl_proxy(upstream, ProxyMode::Strict, REGISTRY_PATH).await;

    let client = reqwest::Client::new();

    let response = client
        .get(format!("http://{}/capabilities", proxy))
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success());

    let body: Value = response.json().await.unwrap();

    assert!(body["capabilities"]["schema_validation"].as_bool().unwrap());
    assert!(body["stypes"].as_array().is_some());
    assert!(body["profiles"].as_array().is_some());

    // Verify calendar event is loaded
    let stypes = body["stypes"].as_array().unwrap();
    assert!(
        stypes
            .iter()
            .any(|s| s.as_str() == Some("org.calendar.Event.v1")),
        "Calendar Event SType should be loaded. Available: {:?}",
        stypes
    );
}

#[tokio::test]
async fn test_ai_alpn_handshake_negotiation() {
    let upstream = start_mock_upstream().await;
    let proxy = start_mpl_proxy(upstream, ProxyMode::Strict, REGISTRY_PATH).await;

    let client = reqwest::Client::new();

    let response = client
        .post(format!("http://{}/.well-known/ai-alpn", proxy))
        .json(&json!({
            "type": "ai-alpn-hello",
            "version": "1.0",
            "stypes": ["org.calendar.Event.v1", "org.finance.InvestmentRecommendation.v1", "org.unknown.Foo.v1"],
            "qom_profiles": ["qom-basic", "qom-strict-argcheck"]
        }))
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success());

    let body: Value = response.json().await.unwrap();
    assert_eq!(body["type"], "ai-alpn-select");

    // Should only contain common STypes (those registered in registry)
    let common_stypes = body["common_stypes"].as_array().unwrap();
    assert!(
        common_stypes.iter().any(|s| s == "org.calendar.Event.v1"),
        "Calendar Event should be negotiated. Common: {:?}",
        common_stypes
    );
    // Unknown SType should not be in common
    assert!(!common_stypes.iter().any(|s| s == "org.unknown.Foo.v1"));

    // Should have selected profile
    assert!(body["selected_profile"].is_string());
}

// =============================================================================
// Pass-through Tests (non-MPL requests)
// =============================================================================

#[tokio::test]
async fn test_non_mpl_request_passes_through() {
    let upstream = start_mock_upstream().await;
    let proxy = start_mpl_proxy(upstream, ProxyMode::Strict, REGISTRY_PATH).await;

    let client = reqwest::Client::new();

    // Send plain JSON without MPL envelope or headers
    let response = client
        .post(format!("http://{}/api/data", proxy))
        .json(&json!({
            "regular": "json",
            "no": "mpl"
        }))
        .send()
        .await
        .unwrap();

    // Should pass through (proxy doesn't require MPL for all requests)
    assert!(
        response.status().is_success(),
        "Non-MPL requests should pass through"
    );
}
