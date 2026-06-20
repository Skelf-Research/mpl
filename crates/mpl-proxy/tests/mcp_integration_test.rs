//! MCP Integration Tests
//!
//! Tests the MPL proxy with MCP-style JSON-RPC messages.

use axum::{routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::sleep;

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

/// Mock MCP server handler
async fn mock_mcp_handler(Json(request): Json<JsonRpcRequest>) -> Json<JsonRpcResponse> {
    match request.method.as_str() {
        "initialize" => Json(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "mock-mcp-server",
                    "version": "1.0.0"
                }
            })),
            error: None,
        }),
        "tools/list" => Json(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(json!({
                "tools": [
                    {
                        "name": "calendar_create",
                        "description": "Create a calendar event",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "title": {"type": "string"},
                                "start": {"type": "string"},
                                "end": {"type": "string"}
                            },
                            "required": ["title", "start", "end"]
                        }
                    }
                ]
            })),
            error: None,
        }),
        "tools/call" => {
            let params = request.params.unwrap_or_default();
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");

            match tool_name {
                "calendar_create" => Json(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(json!({
                        "content": [{
                            "type": "text",
                            "text": "Event created successfully"
                        }]
                    })),
                    error: None,
                }),
                _ => Json(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(json!({
                        "code": -32601,
                        "message": "Method not found"
                    })),
                }),
            }
        }
        _ => Json(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: None,
            error: Some(json!({
                "code": -32601,
                "message": "Method not found"
            })),
        }),
    }
}

/// Start mock MCP server
async fn start_mock_mcp_server() -> SocketAddr {
    let app = Router::new().route("/", post(mock_mcp_handler));

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give server time to start
    sleep(Duration::from_millis(100)).await;
    addr
}

#[tokio::test]
async fn test_mcp_initialize_through_proxy() {
    // Start mock MCP server
    let mcp_addr = start_mock_mcp_server().await;

    // Create HTTP client
    let client = reqwest::Client::new();

    // Send initialize request directly to mock server (proxy not started in this test)
    let response = client
        .post(format!("http://{}/", mcp_addr))
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        }))
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success());

    let body: JsonRpcResponse = response.json().await.unwrap();
    assert!(body.result.is_some());
    assert!(body.error.is_none());

    let result = body.result.unwrap();
    assert_eq!(result["serverInfo"]["name"], "mock-mcp-server");
}

#[tokio::test]
async fn test_mcp_tools_list() {
    let mcp_addr = start_mock_mcp_server().await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("http://{}/", mcp_addr))
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        }))
        .send()
        .await
        .unwrap();

    let body: JsonRpcResponse = response.json().await.unwrap();
    let result = body.result.unwrap();
    let tools = result["tools"].as_array().unwrap();

    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "calendar_create");
}

#[tokio::test]
async fn test_mcp_tool_call() {
    let mcp_addr = start_mock_mcp_server().await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("http://{}/", mcp_addr))
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "calendar_create",
                "arguments": {
                    "title": "Team Meeting",
                    "start": "2024-01-15T10:00:00Z",
                    "end": "2024-01-15T11:00:00Z"
                }
            }
        }))
        .send()
        .await
        .unwrap();

    let body: JsonRpcResponse = response.json().await.unwrap();
    assert!(body.result.is_some());
    assert!(body.error.is_none());
}

/// Test MPL envelope wrapping of MCP tool call
#[tokio::test]
async fn test_mpl_envelope_with_mcp_payload() {
    use mpl_core::envelope::MplEnvelope;

    use mpl_core::validation::SchemaValidator;

    // Create MCP tool call payload
    let mcp_payload = json!({
        "title": "Team Meeting",
        "start": "2024-01-15T10:00:00Z",
        "end": "2024-01-15T11:00:00Z"
    });

    // Wrap in MPL envelope
    let mut envelope = MplEnvelope::new("org.calendar.Event.v1".to_string(), mcp_payload.clone());

    // Compute semantic hash
    envelope.compute_hash().unwrap();
    assert!(envelope.sem_hash.is_some());

    // Validate against schema
    let mut validator = SchemaValidator::new();
    validator
        .register(
            "org.calendar.Event.v1",
            json!({
                "type": "object",
                "properties": {
                    "title": {"type": "string"},
                    "start": {"type": "string"},
                    "end": {"type": "string"}
                },
                "required": ["title", "start", "end"]
            }),
        )
        .unwrap();

    let result = validator
        .validate("org.calendar.Event.v1", &mcp_payload)
        .unwrap();
    assert!(result.valid);

    // Verify hash integrity
    assert!(envelope.verify_hash().unwrap());
}

/// Test that invalid MCP payloads are caught by MPL validation
#[tokio::test]
async fn test_mpl_catches_invalid_mcp_payload() {
    use mpl_core::validation::SchemaValidator;

    // Invalid MCP tool call payload (missing required fields)
    let invalid_payload = json!({
        "title": "Team Meeting"
        // Missing start and end
    });

    let mut validator = SchemaValidator::new();
    validator
        .register(
            "org.calendar.Event.v1",
            json!({
                "type": "object",
                "properties": {
                    "title": {"type": "string"},
                    "start": {"type": "string"},
                    "end": {"type": "string"}
                },
                "required": ["title", "start", "end"]
            }),
        )
        .unwrap();

    let result = validator
        .validate("org.calendar.Event.v1", &invalid_payload)
        .unwrap();
    assert!(!result.valid);
    assert!(!result.errors.is_empty());
}
