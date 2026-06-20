//! A2A Protocol Integration Tests using a2a-rs SDK
//!
//! Tests the MPL proxy with actual A2A protocol implementation,
//! including MPL envelope wrapping, schema validation, and QoM enforcement.

use a2a_rs::{
    adapter::{
        business::DefaultMessageHandler, storage::InMemoryTaskStorage, DefaultRequestProcessor,
        SimpleAgentInfo,
    },
    domain::{Message, TaskState},
    services::AsyncA2AClient,
    HttpClient, HttpServer,
};
use mpl_core::prelude::*;
use std::net::TcpListener;
use uuid::Uuid;

/// Find an available port
fn get_available_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

/// Create a test message with auto-generated ID
fn test_message(text: &str) -> Message {
    Message::user_text(text.to_string(), Uuid::new_v4().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a test server
    fn create_test_server(
        port: u16,
    ) -> HttpServer<
        DefaultRequestProcessor<
            DefaultMessageHandler<InMemoryTaskStorage>,
            InMemoryTaskStorage,
            InMemoryTaskStorage,
        >,
        SimpleAgentInfo,
    > {
        let agent_info = SimpleAgentInfo::new("test-a2a-agent".to_string(), "1.0.0".to_string());

        // InMemoryTaskStorage implements both AsyncTaskManager and AsyncNotificationManager
        let task_storage = InMemoryTaskStorage::new();

        // DefaultMessageHandler needs a task manager
        let message_handler = DefaultMessageHandler::new(task_storage.clone());

        // DefaultRequestProcessor takes: message_handler, task_manager, notification_manager
        // InMemoryTaskStorage can be used for both task_manager and notification_manager
        let processor =
            DefaultRequestProcessor::new(message_handler, task_storage.clone(), task_storage);

        HttpServer::new(processor, agent_info, format!("127.0.0.1:{}", port))
    }

    #[tokio::test]
    async fn test_a2a_send_message_and_create_task() {
        let port = get_available_port();
        let server = create_test_server(port);

        let server_handle = tokio::spawn(async move {
            server.start().await.ok();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let client = HttpClient::new(format!("http://127.0.0.1:{}", port));

        // Send a message to create a task
        let message = test_message("Process expense report for $50 lunch");
        let task_id = "task-001";

        let task = client
            .send_task_message(task_id, &message, None, None)
            .await
            .unwrap();

        // Verify task was created
        assert_eq!(task.id, task_id);

        server_handle.abort();
    }

    #[tokio::test]
    async fn test_a2a_get_task() {
        let port = get_available_port();
        let server = create_test_server(port);

        let server_handle = tokio::spawn(async move {
            server.start().await.ok();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let client = HttpClient::new(format!("http://127.0.0.1:{}", port));

        // Create task
        let message = test_message("Calculate quarterly expenses");
        let task_id = "get-task-001";

        client
            .send_task_message(task_id, &message, None, None)
            .await
            .unwrap();

        // Get task status
        let retrieved_task = client.get_task(task_id, None).await.unwrap();

        // Task should exist
        assert_eq!(retrieved_task.id, task_id);

        server_handle.abort();
    }

    #[tokio::test]
    async fn test_a2a_cancel_task() {
        let port = get_available_port();
        let server = create_test_server(port);

        let server_handle = tokio::spawn(async move {
            server.start().await.ok();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let client = HttpClient::new(format!("http://127.0.0.1:{}", port));

        // Create task
        let message = test_message("Task to be cancelled");
        let task_id = "cancel-task-001";

        client
            .send_task_message(task_id, &message, None, None)
            .await
            .unwrap();

        // Cancel task
        let cancelled = client.cancel_task(task_id).await.unwrap();

        // Verify cancelled state (note: American spelling in the enum)
        assert!(matches!(cancelled.status.state, TaskState::Canceled));

        server_handle.abort();
    }

    #[tokio::test]
    async fn test_a2a_multiple_tasks() {
        let port = get_available_port();
        let server = create_test_server(port);

        let server_handle = tokio::spawn(async move {
            server.start().await.ok();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let client = HttpClient::new(format!("http://127.0.0.1:{}", port));

        // Create multiple tasks
        for i in 1..=3 {
            let message = test_message(&format!("Task number {}", i));
            let task_id = format!("multi-task-{}", i);

            let task = client
                .send_task_message(&task_id, &message, None, None)
                .await
                .unwrap();

            assert_eq!(task.id, task_id);
        }

        // Verify each task can be retrieved
        for i in 1..=3 {
            let task_id = format!("multi-task-{}", i);
            let task = client.get_task(&task_id, None).await.unwrap();
            assert_eq!(task.id, task_id);
        }

        server_handle.abort();
    }

    #[tokio::test]
    async fn test_a2a_task_with_context() {
        let port = get_available_port();
        let server = create_test_server(port);

        let server_handle = tokio::spawn(async move {
            server.start().await.ok();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let client = HttpClient::new(format!("http://127.0.0.1:{}", port));

        // Create initial task
        let message = test_message("Start expense workflow");
        let task_id = "context-task-001";

        let task = client
            .send_task_message(task_id, &message, None, None)
            .await
            .unwrap();

        assert_eq!(task.id, task_id);

        // Send follow-up message to same task (context continuation)
        let followup = test_message("Add receipt for $25 taxi");

        let updated_task = client
            .send_task_message(task_id, &followup, None, None)
            .await
            .unwrap();

        // Task should have message history
        assert_eq!(updated_task.id, task_id);

        server_handle.abort();
    }

    #[tokio::test]
    async fn test_a2a_concurrent_clients() {
        let port = get_available_port();
        let server = create_test_server(port);

        let server_handle = tokio::spawn(async move {
            server.start().await.ok();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let base_url = format!("http://127.0.0.1:{}", port);

        // Create multiple clients
        let client1 = HttpClient::new(base_url.clone());
        let client2 = HttpClient::new(base_url.clone());

        // Both clients create tasks
        let msg1 = test_message("Client 1 task");
        let msg2 = test_message("Client 2 task");

        let task1 = client1
            .send_task_message("client1-task", &msg1, None, None)
            .await
            .unwrap();

        let task2 = client2
            .send_task_message("client2-task", &msg2, None, None)
            .await
            .unwrap();

        assert_eq!(task1.id, "client1-task");
        assert_eq!(task2.id, "client2-task");

        // Each client can retrieve tasks created by the other
        let t1 = client2.get_task("client1-task", None).await.unwrap();
        let t2 = client1.get_task("client2-task", None).await.unwrap();

        assert_eq!(t1.id, "client1-task");
        assert_eq!(t2.id, "client2-task");

        server_handle.abort();
    }

    #[tokio::test]
    async fn test_a2a_task_states() {
        let port = get_available_port();
        let server = create_test_server(port);

        let server_handle = tokio::spawn(async move {
            server.start().await.ok();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let client = HttpClient::new(format!("http://127.0.0.1:{}", port));

        // Create task
        let message = test_message("Check task states");
        let task_id = "state-task-001";

        let task = client
            .send_task_message(task_id, &message, None, None)
            .await
            .unwrap();

        // New task should be in submitted or working state
        assert!(
            matches!(task.status.state, TaskState::Submitted | TaskState::Working),
            "Task should be in initial state"
        );

        server_handle.abort();
    }
}

/// MPL Envelope Integration Tests
/// Tests wrapping A2A messages with MPL semantic envelopes
#[cfg(test)]
mod mpl_integration_tests {
    use super::*;
    use mpl_core::qom::QomMetrics;
    use serde_json::json;

    /// Test creating MPL envelope for A2A task message
    #[test]
    fn test_mpl_envelope_for_a2a_task() {
        let task_payload = json!({
            "task_id": "task-001",
            "description": "Process expense report",
            "priority": "high"
        });

        let envelope =
            MplEnvelope::new("agent.Task.v1", task_payload.clone()).with_profile("qom-basic");

        assert_eq!(envelope.stype, "agent.Task.v1");
        assert_eq!(envelope.profile, Some("qom-basic".to_string()));
        assert_eq!(envelope.payload, task_payload);
    }

    /// Test envelope with semantic hash computation
    #[test]
    fn test_mpl_envelope_with_hash() {
        let payload = json!({
            "action": "approve",
            "amount": 5000,
            "currency": "USD"
        });

        let mut envelope = MplEnvelope::new("org.finance.Approval.v1", payload.clone());
        envelope.compute_hash().unwrap();

        assert!(envelope.sem_hash.is_some());
        let hash = envelope.sem_hash.unwrap();
        assert!(hash.starts_with("b3:"));
    }

    /// Test envelope with QoM report
    #[test]
    fn test_mpl_envelope_with_qom_report() {
        let payload = json!({
            "recommendation": "buy",
            "symbol": "VOO"
        });

        let metrics = QomMetrics::schema_valid().with_instruction_compliance(0.98);
        let report = QomReport::pass("qom-basic", metrics);

        let envelope =
            MplEnvelope::new("org.finance.Recommendation.v1", payload).with_qom_report(report);

        assert!(envelope.qom_report.is_some());
        let qom = envelope.qom_report.as_ref().unwrap();
        assert_eq!(qom.profile, "qom-basic");
        assert!(qom.meets_profile);
    }

    /// Test A2A message wrapped with provenance
    #[test]
    fn test_mpl_envelope_with_provenance() {
        use mpl_core::envelope::Provenance;

        let payload = json!({
            "step": "verify_identity",
            "status": "completed"
        });

        let provenance = Provenance::new("task.verify.v1")
            .with_agent("agent://verifier")
            .with_inputs(vec!["ctx:user-input#1".to_string()])
            .with_policy("policy.ref#kyc-v1");

        let envelope =
            MplEnvelope::new("org.workflow.Step.v1", payload).with_provenance(provenance);

        assert!(envelope.provenance.is_some());
        let prov = envelope.provenance.as_ref().unwrap();
        assert_eq!(prov.agent, Some("agent://verifier".to_string()));
        assert_eq!(prov.policy_ref, Some("policy.ref#kyc-v1".to_string()));
    }

    /// Test AI-ALPN handshake for A2A peer negotiation
    #[test]
    fn test_a2a_peer_handshake() {
        // Peer A sends hello
        let hello = ClientHello::new()
            .with_protocols(vec!["a2a/0.3".to_string()])
            .with_stypes(vec![
                "agent.TaskPlan.v1".to_string(),
                "org.finance.Query.v1".to_string(),
            ])
            .with_profile("qom-strict-argcheck");

        assert!(hello.protocols.contains(&"a2a/0.3".to_string()));
        assert_eq!(hello.stypes.len(), 2);

        // Peer B responds with selection
        let select = ServerSelect::success()
            .with_protocol("a2a/0.3")
            .with_stypes(vec!["agent.TaskPlan.v1".to_string()])
            .with_profile("qom-basic");

        assert_eq!(select.protocol, Some("a2a/0.3".to_string()));
        assert_eq!(select.profile, Some("qom-basic".to_string()));
    }

    /// Test policy enforcement for A2A messages
    #[test]
    fn test_a2a_policy_enforcement() {
        use mpl_core::policy::{Operation, Policy, StypePattern};
        use mpl_core::stype::SType;

        let mut engine = PolicyEngine::new();

        // Add policy requiring QoM for financial operations
        let policy = Policy::new("finance-qom-required")
            .with_stype_pattern(StypePattern::namespace_domain("org", "finance"))
            .with_operations([Operation::Execute])
            .with_qom_override("qom-strict-argcheck");

        engine.add_policy(policy);

        // Test financial message
        let stype = SType::parse("org.finance.Transfer.v1").unwrap();
        let ctx = PolicyContext::new(stype, Operation::Execute).with_principal("agent://executor");

        let decision = engine.evaluate(&ctx);
        assert!(decision.is_allowed());
    }

    /// Test schema validation for A2A task payload
    #[test]
    fn test_a2a_schema_validation() {
        let schema = json!({
            "type": "object",
            "properties": {
                "task_id": {"type": "string"},
                "description": {"type": "string"},
                "priority": {"type": "string", "enum": ["low", "medium", "high"]}
            },
            "required": ["task_id", "description"]
        });

        let mut validator = SchemaValidator::new();
        validator.register("agent.Task.v1", schema).unwrap();

        // Valid payload
        let valid = json!({
            "task_id": "task-001",
            "description": "Process report",
            "priority": "high"
        });
        let result = validator.validate("agent.Task.v1", &valid).unwrap();
        assert!(result.valid);

        // Invalid payload (missing required field)
        let invalid = json!({
            "task_id": "task-002"
        });
        let result = validator.validate("agent.Task.v1", &invalid).unwrap();
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
    }

    /// Test QoM profile evaluation for A2A workflow
    #[test]
    fn test_a2a_qom_profile_evaluation() {
        use mpl_core::qom::QomProfile;

        // Test basic profile with passing metrics
        let basic_profile = QomProfile::basic();
        let metrics = QomMetrics::schema_valid().with_instruction_compliance(0.95);
        let eval = basic_profile.evaluate(&metrics);
        assert!(eval.meets_profile);

        // Test strict profile with threshold breach
        let strict_profile = QomProfile::strict_argcheck();
        let strict_metrics = QomMetrics::schema_valid().with_instruction_compliance(0.92); // Below 0.97 threshold
        let strict_eval = strict_profile.evaluate(&strict_metrics);
        assert!(!strict_eval.meets_profile);
    }

    /// Test error handling for A2A semantic failures
    #[test]
    fn test_a2a_error_taxonomy() {
        use mpl_core::error::SchemaError;

        // Schema fidelity error
        let schema_error = MplError::SchemaFidelity {
            message: "Invalid task payload structure".to_string(),
            stype: "agent.Task.v1".to_string(),
            errors: vec![SchemaError {
                path: "/priority".to_string(),
                message: "expected string, got number".to_string(),
                expected: Some("string".to_string()),
                actual: Some("number".to_string()),
            }],
            hints: vec!["Check priority field type".to_string()],
        };

        assert!(matches!(schema_error, MplError::SchemaFidelity { .. }));

        // Unknown SType error
        let stype_error = MplError::UnknownStype {
            stype: "custom.Unknown.v1".to_string(),
            suggestions: vec!["org.workflow.Step.v1".to_string()],
        };

        assert!(matches!(stype_error, MplError::UnknownStype { .. }));
    }

    /// Test envelope serialization for A2A transport
    #[test]
    fn test_envelope_json_serialization() {
        let mut envelope =
            MplEnvelope::new("agent.Task.v1", json!({"task_id": "001"})).with_profile("qom-basic");
        envelope.compute_hash().unwrap();

        // Serialize to JSON
        let json_str = serde_json::to_string(&envelope).unwrap();
        assert!(json_str.contains("agent.Task.v1"));
        assert!(json_str.contains("qom-basic"));

        // Deserialize back
        let parsed: MplEnvelope = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.stype, envelope.stype);
        assert_eq!(parsed.profile, envelope.profile);
    }

    /// Test concurrent envelope creation (thread safety)
    #[test]
    fn test_concurrent_envelope_creation() {
        use std::thread;

        let handles: Vec<_> = (0..10)
            .map(|i| {
                thread::spawn(move || {
                    let payload = json!({"index": i});
                    let mut envelope = MplEnvelope::new("test.Concurrent.v1", payload);
                    envelope.compute_hash().unwrap();

                    // Each envelope should have a unique ID and hash
                    assert!(!envelope.id.is_empty());
                    assert!(envelope.sem_hash.is_some());
                    envelope.id
                })
            })
            .collect();

        let ids: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        // All IDs should be unique
        let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(unique_ids.len(), 10);
    }

    /// Test envelope hash verification
    #[test]
    fn test_envelope_hash_verification() {
        let payload = json!({"data": "test"});
        let mut envelope = MplEnvelope::new("test.Data.v1", payload);
        envelope.compute_hash().unwrap();

        // Verify hash is valid
        assert!(envelope.verify_hash().unwrap());

        // Modify payload and verify hash fails
        envelope.payload = json!({"data": "modified"});
        assert!(!envelope.verify_hash().unwrap());
    }

    /// Test handshake with downgrades
    #[test]
    fn test_a2a_handshake_with_downgrades() {
        use mpl_core::handshake::{Downgrade, DowngradeCategory};

        let select = ServerSelect::success()
            .with_protocol("a2a/0.3")
            .with_profile("qom-basic") // Downgraded from strict
            .with_downgrade(
                Downgrade::new(
                    DowngradeCategory::Profile,
                    "qom-strict-argcheck",
                    "IC assertions not available",
                )
                .with_selected("qom-basic"),
            );

        assert!(select.success);
        assert_eq!(select.downgrades.len(), 1);
        assert_eq!(select.downgrades[0].requested, "qom-strict-argcheck");
        assert_eq!(select.downgrades[0].selected, Some("qom-basic".to_string()));
    }
}
