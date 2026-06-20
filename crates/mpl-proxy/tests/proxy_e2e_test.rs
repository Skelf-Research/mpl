//! End-to-End Proxy Tests
//!
//! Tests the full flow: Client -> MPL Proxy -> MCP Server

use axum::{routing::post, Json, Router};
use mpl_core::envelope::MplEnvelope;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::sleep;

/// Registry path relative to the crate directory (tests run from crates/mpl-proxy)
const REGISTRY_PATH: &str = "../../registry";

/// Track requests received by mock server
struct RequestTracker {
    count: AtomicU32,
}

impl RequestTracker {
    fn new() -> Self {
        Self {
            count: AtomicU32::new(0),
        }
    }

    fn increment(&self) {
        self.count.fetch_add(1, Ordering::SeqCst);
    }

    fn get(&self) -> u32 {
        self.count.load(Ordering::SeqCst)
    }
}

/// MCP JSON-RPC Request
#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Value,
    method: String,
    params: Option<Value>,
}

/// MCP JSON-RPC Response
#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

async fn mcp_handler(
    tracker: Arc<RequestTracker>,
    Json(request): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    tracker.increment();

    match request.method.as_str() {
        "tools/call" => {
            let params = request.params.unwrap_or_default();
            Json(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Processed: {}", params)
                    }]
                })),
                error: None,
            })
        }
        _ => Json(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(json!({"status": "ok"})),
            error: None,
        }),
    }
}

/// Start mock MCP server with request tracking
async fn start_tracked_mcp_server(tracker: Arc<RequestTracker>) -> SocketAddr {
    let t1 = tracker.clone();
    let t2 = tracker.clone();

    let app = Router::new()
        .route(
            "/",
            post(move |body| {
                let t = t1.clone();
                async move { mcp_handler(t, body).await }
            }),
        )
        .route(
            "/*path",
            post(move |body| {
                let t = t2.clone();
                async move { mcp_handler(t, body).await }
            }),
        );

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    sleep(Duration::from_millis(50)).await;
    addr
}

/// Start MPL proxy pointing to upstream
async fn start_mpl_proxy(upstream_addr: SocketAddr, registry_path: &str) -> SocketAddr {
    use mpl_proxy::config::{
        MplConfig, ObservabilityConfig, ProxyConfig, ProxyMode, TransportConfig,
    };
    use mpl_proxy::handlers;
    use mpl_proxy::proxy::ProxyState;
    use std::sync::Arc;

    let config = ProxyConfig {
        transport: TransportConfig {
            listen: "127.0.0.1:0".to_string(),
            upstream: upstream_addr.to_string(),
            ..Default::default()
        },
        mpl: MplConfig {
            registry: registry_path.to_string(),
            mode: ProxyMode::Transparent,
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

#[tokio::test]
async fn test_proxy_health_endpoint() {
    let tracker = Arc::new(RequestTracker::new());
    let mcp_addr = start_tracked_mcp_server(tracker.clone()).await;
    let proxy_addr = start_mpl_proxy(mcp_addr, REGISTRY_PATH).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{}/health", proxy_addr))
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success());
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["status"], "healthy");
}

#[tokio::test]
async fn test_proxy_capabilities_endpoint() {
    let tracker = Arc::new(RequestTracker::new());
    let mcp_addr = start_tracked_mcp_server(tracker.clone()).await;
    let proxy_addr = start_mpl_proxy(mcp_addr, REGISTRY_PATH).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{}/capabilities", proxy_addr))
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success());
    let body: Value = response.json().await.unwrap();
    assert!(body["capabilities"]["schema_validation"].as_bool().unwrap());
    assert!(body["capabilities"]["qom_evaluation"].as_bool().unwrap());
}

#[tokio::test]
async fn test_proxy_ai_alpn_handshake() {
    let tracker = Arc::new(RequestTracker::new());
    let mcp_addr = start_tracked_mcp_server(tracker.clone()).await;
    let proxy_addr = start_mpl_proxy(mcp_addr, REGISTRY_PATH).await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://{}/.well-known/ai-alpn", proxy_addr))
        .json(&json!({
            "type": "ai-alpn-hello",
            "version": "1.0",
            "stypes": ["org.calendar.Event.v1", "org.agent.TaskPlan.v1"],
            "qom_profiles": ["qom-basic"]
        }))
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success());
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["type"], "ai-alpn-select");
    assert!(body["common_stypes"].as_array().is_some());
}

#[tokio::test]
async fn test_proxy_forwards_mcp_request() {
    let tracker = Arc::new(RequestTracker::new());
    let mcp_addr = start_tracked_mcp_server(tracker.clone()).await;
    let proxy_addr = start_mpl_proxy(mcp_addr, REGISTRY_PATH).await;

    let client = reqwest::Client::new();

    // Send MCP request through proxy
    let response = client
        .post(format!("http://{}/rpc", proxy_addr))
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "test_tool",
                "arguments": {"key": "value"}
            }
        }))
        .send()
        .await
        .unwrap();

    // Request should be forwarded to upstream
    let status = response.status();
    println!("Response status: {}", status);

    // Verify request reached upstream server
    assert!(tracker.get() >= 1, "Request should reach upstream server");
}

#[tokio::test]
async fn test_proxy_with_mpl_envelope() {
    let tracker = Arc::new(RequestTracker::new());
    let mcp_addr = start_tracked_mcp_server(tracker.clone()).await;
    let proxy_addr = start_mpl_proxy(mcp_addr, REGISTRY_PATH).await;

    let client = reqwest::Client::new();

    // Create MPL envelope
    let mut envelope = MplEnvelope::new(
        "org.calendar.Event.v1".to_string(),
        json!({
            "title": "Team Meeting",
            "start": "2024-01-15T10:00:00Z",
            "end": "2024-01-15T11:00:00Z"
        }),
    );
    envelope.compute_hash().unwrap();

    // Send through proxy
    let response = client
        .post(format!("http://{}/mcp", proxy_addr))
        .header("X-MPL-SType", "org.calendar.Event.v1")
        .json(&envelope)
        .send()
        .await
        .unwrap();

    // Check QoM result header is present
    let qom_result = response.headers().get("X-MPL-QoM-Result");
    assert!(qom_result.is_some(), "QoM result header should be present");
}
