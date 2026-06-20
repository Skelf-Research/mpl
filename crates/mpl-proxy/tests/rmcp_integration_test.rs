//! Real MCP Protocol Integration Tests using rmcp SDK
//!
//! Tests the MPL proxy with actual MCP protocol implementation.

use rmcp::{
    handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::{
        CallToolResult, Content, Implementation, InitializeResult, ProtocolVersion,
        ServerCapabilities,
    },
    tool, tool_router, ServerHandler, ServiceExt,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Calendar event tool input
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct CreateEventInput {
    /// Event title
    title: String,
    /// Start time in ISO 8601 format
    start: String,
    /// End time in ISO 8601 format
    end: String,
    /// Optional description
    #[serde(default)]
    description: Option<String>,
}

/// Counter increment input
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct IncrementInput {
    /// Amount to increment by
    #[serde(default = "default_amount")]
    amount: i32,
}

fn default_amount() -> i32 {
    1
}

/// Echo input
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct EchoInput {
    /// Message to echo
    message: String,
}

/// Mock MCP Server with tools
#[derive(Clone)]
pub struct MockMcpServer {
    counter: Arc<Mutex<i32>>,
    events: Arc<Mutex<Vec<String>>>,
    tool_router: ToolRouter<Self>,
}

impl Default for MockMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl MockMcpServer {
    pub fn new() -> Self {
        Self {
            counter: Arc::new(Mutex::new(0)),
            events: Arc::new(Mutex::new(Vec::new())),
            tool_router: Self::tool_router(),
        }
    }
}

/// Tool implementations
#[tool_router]
impl MockMcpServer {
    /// Create a calendar event
    #[tool(description = "Create a new calendar event")]
    async fn calendar_create(
        &self,
        input: Parameters<CreateEventInput>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        // Parameters is a newtype wrapper, access inner with .0
        let event_str = format!(
            "Event: {} from {} to {}",
            input.0.title, input.0.start, input.0.end
        );

        let mut events = self.events.lock().await;
        events.push(event_str.clone());

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Created event: {}",
            input.0.title
        ))]))
    }

    /// Increment a counter
    #[tool(description = "Increment the counter by a specified amount")]
    async fn increment(
        &self,
        input: Parameters<IncrementInput>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let mut counter = self.counter.lock().await;
        *counter += input.0.amount;
        let new_value = *counter;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Counter is now: {}",
            new_value
        ))]))
    }

    /// Get current counter value
    #[tool(description = "Get the current counter value")]
    async fn get_counter(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        let counter = self.counter.lock().await;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Counter value: {}",
            *counter
        ))]))
    }

    /// Echo back input for testing
    #[tool(description = "Echo back the provided message")]
    async fn echo(&self, input: Parameters<EchoInput>) -> Result<CallToolResult, rmcp::ErrorData> {
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Echo: {}",
            input.0.message
        ))]))
    }
}

impl ServerHandler for MockMcpServer {
    fn get_info(&self) -> InitializeResult {
        InitializeResult {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "mock-mcp-server".to_string(),
                version: "1.0.0".to_string(),
                ..Default::default()
            },
            instructions: None,
        }
    }

    async fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<rmcp::model::ListToolsResult, rmcp::ErrorData> {
        let tools = self.tool_router.list_all();
        Ok(rmcp::model::ListToolsResult {
            tools,
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParam,
        context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let tool_context =
            rmcp::handler::server::tool::ToolCallContext::new(self, request, context);
        self.tool_router.call(tool_context).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::transport::IntoTransport;
    use tokio::io::duplex;

    #[tokio::test]
    async fn test_mcp_server_initialize() {
        let server = MockMcpServer::new();
        let (client_stream, server_stream) = duplex(8192);

        // Start server in background - serve returns a RunningService that we need to keep alive
        let server_handle = tokio::spawn(async move {
            let transport = server_stream.into_transport();
            let running_server = server.serve(transport).await.unwrap();
            // Keep server running until client disconnects
            running_server.waiting().await.ok();
        });

        // Create client
        let transport = client_stream.into_transport();
        let client = ().serve(transport).await.unwrap();

        // Verify server info - peer_info returns Option<&InitializeResult>
        let peer = client.peer_info().unwrap();
        assert_eq!(peer.server_info.name, "mock-mcp-server");
        assert_eq!(peer.server_info.version, "1.0.0");

        // Cleanup
        client.cancel().await.ok();
        let _ = server_handle.await;
    }

    #[tokio::test]
    async fn test_mcp_list_tools() {
        let server = MockMcpServer::new();
        let (client_stream, server_stream) = duplex(8192);

        let server_handle = tokio::spawn(async move {
            let transport = server_stream.into_transport();
            let running_server = server.serve(transport).await.unwrap();
            running_server.waiting().await.ok();
        });

        let transport = client_stream.into_transport();
        let client = ().serve(transport).await.unwrap();

        // List available tools
        let tools = client.list_all_tools().await.unwrap();

        // Should have our defined tools
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(
            tool_names.contains(&"calendar_create"),
            "Should have calendar_create tool"
        );
        assert!(
            tool_names.contains(&"increment"),
            "Should have increment tool"
        );
        assert!(
            tool_names.contains(&"get_counter"),
            "Should have get_counter tool"
        );
        assert!(tool_names.contains(&"echo"), "Should have echo tool");

        client.cancel().await.ok();
        let _ = server_handle.await;
    }

    #[tokio::test]
    async fn test_mcp_call_echo_tool() {
        let server = MockMcpServer::new();
        let (client_stream, server_stream) = duplex(8192);

        let server_handle = tokio::spawn(async move {
            let transport = server_stream.into_transport();
            let running_server = server.serve(transport).await.unwrap();
            running_server.waiting().await.ok();
        });

        let transport = client_stream.into_transport();
        let client = ().serve(transport).await.unwrap();

        // Call echo tool
        let result = client
            .call_tool(rmcp::model::CallToolRequestParam {
                name: "echo".into(),
                arguments: Some(
                    serde_json::json!({"message": "Hello MCP!"})
                        .as_object()
                        .unwrap()
                        .clone(),
                ),
            })
            .await
            .unwrap();

        // Verify response
        assert!(!result.is_error.unwrap_or(false));
        let text = result.content[0].as_text().unwrap();
        assert!(text.text.contains("Echo: Hello MCP!"));

        client.cancel().await.ok();
        let _ = server_handle.await;
    }

    #[tokio::test]
    async fn test_mcp_call_counter_tools() {
        let server = MockMcpServer::new();
        let (client_stream, server_stream) = duplex(8192);

        let server_handle = tokio::spawn(async move {
            let transport = server_stream.into_transport();
            let running_server = server.serve(transport).await.unwrap();
            running_server.waiting().await.ok();
        });

        let transport = client_stream.into_transport();
        let client = ().serve(transport).await.unwrap();

        // Increment counter
        let result = client
            .call_tool(rmcp::model::CallToolRequestParam {
                name: "increment".into(),
                arguments: Some(
                    serde_json::json!({"amount": 5})
                        .as_object()
                        .unwrap()
                        .clone(),
                ),
            })
            .await
            .unwrap();

        let text = result.content[0].as_text().unwrap();
        assert!(text.text.contains("Counter is now: 5"));

        // Get counter
        let result = client
            .call_tool(rmcp::model::CallToolRequestParam {
                name: "get_counter".into(),
                arguments: None,
            })
            .await
            .unwrap();

        let text = result.content[0].as_text().unwrap();
        assert!(text.text.contains("Counter value: 5"));

        // Increment again
        let result = client
            .call_tool(rmcp::model::CallToolRequestParam {
                name: "increment".into(),
                arguments: Some(
                    serde_json::json!({"amount": 3})
                        .as_object()
                        .unwrap()
                        .clone(),
                ),
            })
            .await
            .unwrap();

        let text = result.content[0].as_text().unwrap();
        assert!(text.text.contains("Counter is now: 8"));

        client.cancel().await.ok();
        let _ = server_handle.await;
    }

    #[tokio::test]
    async fn test_mcp_call_calendar_create() {
        let server = MockMcpServer::new();
        let (client_stream, server_stream) = duplex(8192);

        let server_handle = tokio::spawn(async move {
            let transport = server_stream.into_transport();
            let running_server = server.serve(transport).await.unwrap();
            running_server.waiting().await.ok();
        });

        let transport = client_stream.into_transport();
        let client = ().serve(transport).await.unwrap();

        // Create calendar event
        let result = client
            .call_tool(rmcp::model::CallToolRequestParam {
                name: "calendar_create".into(),
                arguments: Some(
                    serde_json::json!({
                        "title": "Team Standup",
                        "start": "2024-01-15T09:00:00Z",
                        "end": "2024-01-15T09:30:00Z",
                        "description": "Daily team sync"
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
            })
            .await
            .unwrap();

        assert!(!result.is_error.unwrap_or(false));
        let text = result.content[0].as_text().unwrap();
        assert!(text.text.contains("Created event: Team Standup"));

        client.cancel().await.ok();
        let _ = server_handle.await;
    }

    #[tokio::test]
    async fn test_mcp_tool_schema_validation() {
        let server = MockMcpServer::new();
        let (client_stream, server_stream) = duplex(8192);

        let server_handle = tokio::spawn(async move {
            let transport = server_stream.into_transport();
            let running_server = server.serve(transport).await.unwrap();
            running_server.waiting().await.ok();
        });

        let transport = client_stream.into_transport();
        let client = ().serve(transport).await.unwrap();

        // Get tools and verify schema
        let tools = client.list_all_tools().await.unwrap();

        let calendar_tool = tools
            .iter()
            .find(|t| t.name.as_ref() == "calendar_create")
            .unwrap();

        // Verify input schema exists and is valid JSON
        let schema = &calendar_tool.input_schema;

        // Should have properties defined
        assert!(
            schema.contains_key("properties")
                || schema.contains_key("$defs")
                || schema.contains_key("title")
        );

        client.cancel().await.ok();
        let _ = server_handle.await;
    }

    #[tokio::test]
    async fn test_mcp_multiple_clients_shared_state() {
        let server = MockMcpServer::new();

        // We need separate server instances since each connection is 1:1
        let server1 = server.clone();
        let server2 = server.clone();

        let (client1_stream, server1_stream) = duplex(8192);
        let (client2_stream, server2_stream) = duplex(8192);

        let server1_handle = tokio::spawn(async move {
            let transport = server1_stream.into_transport();
            let running_server = server1.serve(transport).await.unwrap();
            running_server.waiting().await.ok();
        });

        let server2_handle = tokio::spawn(async move {
            let transport = server2_stream.into_transport();
            let running_server = server2.serve(transport).await.unwrap();
            running_server.waiting().await.ok();
        });

        let transport1 = client1_stream.into_transport();
        let transport2 = client2_stream.into_transport();

        let client1 = ().serve(transport1).await.unwrap();
        let client2 = ().serve(transport2).await.unwrap();

        // Client 1 increments
        let result = client1
            .call_tool(rmcp::model::CallToolRequestParam {
                name: "increment".into(),
                arguments: Some(
                    serde_json::json!({"amount": 10})
                        .as_object()
                        .unwrap()
                        .clone(),
                ),
            })
            .await
            .unwrap();

        let text = result.content[0].as_text().unwrap();
        assert!(text.text.contains("Counter is now: 10"));

        // Client 2 should see same counter (shared state)
        let result = client2
            .call_tool(rmcp::model::CallToolRequestParam {
                name: "get_counter".into(),
                arguments: None,
            })
            .await
            .unwrap();

        let text = result.content[0].as_text().unwrap();
        assert!(text.text.contains("Counter value: 10"));

        client1.cancel().await.ok();
        client2.cancel().await.ok();
        let _ = server1_handle.await;
        let _ = server2_handle.await;
    }
}
