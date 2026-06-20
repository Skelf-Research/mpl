//! Conformance test suite for MPL protocol
//!
//! This test suite validates compliance with the MPL specification.
//! It covers SType parsing, validation, QoM evaluation, policy enforcement,
//! envelope handling, and error taxonomy.

use mpl_core::envelope::MplEnvelope;
use mpl_core::error::{MplError, MplErrorCode};
use mpl_core::handshake::{ClientHello, ServerSelect};
use mpl_core::hash::{canonicalize, semantic_hash, verify_hash};
use mpl_core::policy::{
    AccessControlRule, AccessDefault, Constraint, ConstraintExpr, Operation, Policy, PolicyContext,
    PolicyEngine, StypePattern, VersionConstraint,
};
use mpl_core::qom::{QomMetrics, QomProfile, QomReport};
use mpl_core::stype::SType;
use mpl_core::validation::SchemaValidator;
use serde_json::json;
use std::collections::HashSet;

// =============================================================================
// Section 1: SType Conformance Tests
// =============================================================================

mod stype_tests {
    use super::*;

    #[test]
    fn test_stype_simple_format() {
        let stype = SType::parse("namespace.domain.Name.v1").unwrap();
        assert_eq!(stype.namespace, "namespace");
        assert_eq!(stype.domain, "domain");
        assert_eq!(stype.name, "Name");
        assert_eq!(stype.major_version, 1);
    }

    #[test]
    fn test_stype_nested_namespace() {
        let stype = SType::parse("com.example.corp.finance.Report.v3").unwrap();
        assert_eq!(stype.namespace, "com.example.corp");
        assert_eq!(stype.domain, "finance");
        assert_eq!(stype.name, "Report");
        assert_eq!(stype.major_version, 3);
    }

    #[test]
    fn test_stype_urn_format() {
        let stype = SType::parse("urn:stype:org.calendar.Event.v1").unwrap();
        assert_eq!(stype.urn(), "urn:stype:org.calendar.Event.v1");
    }

    #[test]
    fn test_stype_registry_path() {
        let stype = SType::parse("eval.rag.RAGQuery.v1").unwrap();
        // Registry path includes leading slash and schema.json
        assert!(stype
            .registry_path()
            .contains("stypes/eval/rag/RAGQuery/v1"));
    }

    #[test]
    fn test_stype_roundtrip() {
        let original = "org.workflow.Step.v2";
        let stype = SType::parse(original).unwrap();
        assert_eq!(stype.id(), original);
    }

    #[test]
    fn test_stype_invalid_format() {
        // Missing version
        assert!(SType::parse("namespace.domain.Name").is_err());
        // Invalid version format
        assert!(SType::parse("namespace.domain.Name.1").is_err());
        // Too few parts
        assert!(SType::parse("domain.Name.v1").is_err());
    }

    #[test]
    fn test_stype_equality() {
        let s1 = SType::parse("org.test.Type.v1").unwrap();
        let s2 = SType::parse("org.test.Type.v1").unwrap();
        let s3 = SType::parse("org.test.Type.v2").unwrap();

        assert_eq!(s1.id(), s2.id());
        assert_ne!(s1.id(), s3.id());
    }
}

// =============================================================================
// Section 2: Schema Validation Conformance Tests
// =============================================================================

mod validation_tests {
    use super::*;

    fn create_event_validator() -> SchemaValidator {
        let mut validator = SchemaValidator::new();
        validator
            .register(
                "org.calendar.Event.v1",
                json!({
                    "type": "object",
                    "properties": {
                        "title": {"type": "string", "minLength": 1},
                        "start": {"type": "string", "format": "date-time"},
                        "end": {"type": "string", "format": "date-time"},
                        "location": {"type": "string"},
                        "attendees": {
                            "type": "array",
                            "items": {"type": "string", "format": "email"}
                        }
                    },
                    "required": ["title", "start", "end"]
                }),
            )
            .unwrap();
        validator
    }

    #[test]
    fn test_valid_minimal_payload() {
        let validator = create_event_validator();
        let payload = json!({
            "title": "Meeting",
            "start": "2024-01-15T10:00:00Z",
            "end": "2024-01-15T11:00:00Z"
        });
        let result = validator
            .validate("org.calendar.Event.v1", &payload)
            .unwrap();
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_valid_full_payload() {
        let validator = create_event_validator();
        let payload = json!({
            "title": "Team Standup",
            "start": "2024-01-15T09:00:00Z",
            "end": "2024-01-15T09:30:00Z",
            "location": "Conference Room A",
            "attendees": ["alice@example.com", "bob@example.com"]
        });
        let result = validator
            .validate("org.calendar.Event.v1", &payload)
            .unwrap();
        assert!(result.valid);
    }

    #[test]
    fn test_missing_required_field() {
        let validator = create_event_validator();
        let payload = json!({
            "title": "Meeting",
            "start": "2024-01-15T10:00:00Z"
            // Missing "end"
        });
        let result = validator
            .validate("org.calendar.Event.v1", &payload)
            .unwrap();
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("end") || e.path.contains("end")));
    }

    #[test]
    fn test_invalid_type() {
        let validator = create_event_validator();
        let payload = json!({
            "title": 123, // Should be string
            "start": "2024-01-15T10:00:00Z",
            "end": "2024-01-15T11:00:00Z"
        });
        let result = validator
            .validate("org.calendar.Event.v1", &payload)
            .unwrap();
        assert!(!result.valid);
    }

    #[test]
    fn test_empty_required_string() {
        let validator = create_event_validator();
        let payload = json!({
            "title": "", // Empty string, minLength: 1
            "start": "2024-01-15T10:00:00Z",
            "end": "2024-01-15T11:00:00Z"
        });
        let result = validator
            .validate("org.calendar.Event.v1", &payload)
            .unwrap();
        assert!(!result.valid);
    }

    #[test]
    fn test_unknown_stype() {
        let validator = create_event_validator();
        let payload = json!({"data": "test"});
        let result = validator.validate("unknown.Type.v1", &payload);
        assert!(result.is_err());
    }

    #[test]
    fn test_extra_properties_allowed() {
        let validator = create_event_validator();
        let payload = json!({
            "title": "Meeting",
            "start": "2024-01-15T10:00:00Z",
            "end": "2024-01-15T11:00:00Z",
            "customField": "custom value" // Extra property
        });
        let result = validator
            .validate("org.calendar.Event.v1", &payload)
            .unwrap();
        // By default, JSON Schema allows additional properties
        assert!(result.valid);
    }

    #[test]
    fn test_nested_object_validation() {
        let mut validator = SchemaValidator::new();
        validator
            .register(
                "data.record.Record.v1",
                json!({
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"},
                        "metadata": {
                            "type": "object",
                            "properties": {
                                "created": {"type": "string"},
                                "tags": {"type": "array", "items": {"type": "string"}}
                            },
                            "required": ["created"]
                        }
                    },
                    "required": ["id", "metadata"]
                }),
            )
            .unwrap();

        let valid_payload = json!({
            "id": "rec-123",
            "metadata": {
                "created": "2024-01-15",
                "tags": ["important", "urgent"]
            }
        });
        assert!(
            validator
                .validate("data.record.Record.v1", &valid_payload)
                .unwrap()
                .valid
        );

        let invalid_payload = json!({
            "id": "rec-123",
            "metadata": {
                "tags": ["important"] // Missing "created"
            }
        });
        assert!(
            !validator
                .validate("data.record.Record.v1", &invalid_payload)
                .unwrap()
                .valid
        );
    }
}

// =============================================================================
// Section 3: QoM Conformance Tests
// =============================================================================

mod qom_tests {
    use super::*;

    #[test]
    fn test_qom_basic_profile_pass() {
        let profile = QomProfile::basic();
        let metrics = QomMetrics::schema_valid();
        let eval = profile.evaluate(&metrics);

        assert!(eval.meets_profile);
        assert_eq!(eval.profile, "qom-basic");
        assert!(eval.failures.is_empty());
    }

    #[test]
    fn test_qom_basic_profile_fail() {
        let profile = QomProfile::basic();
        let metrics = QomMetrics::schema_invalid();
        let eval = profile.evaluate(&metrics);

        assert!(!eval.meets_profile);
        assert_eq!(eval.failures.len(), 1);
        assert_eq!(eval.failures[0].metric, "schema_fidelity");
    }

    #[test]
    fn test_qom_strict_argcheck_pass() {
        let profile = QomProfile::strict_argcheck();
        let metrics = QomMetrics::schema_valid().with_instruction_compliance(0.98);
        let eval = profile.evaluate(&metrics);

        assert!(eval.meets_profile);
    }

    #[test]
    fn test_qom_strict_argcheck_fail_ic() {
        let profile = QomProfile::strict_argcheck();
        let metrics = QomMetrics::schema_valid().with_instruction_compliance(0.90);
        let eval = profile.evaluate(&metrics);

        assert!(!eval.meets_profile);
        assert!(eval
            .failures
            .iter()
            .any(|f| f.metric == "instruction_compliance"));
    }

    #[test]
    fn test_qom_outcome_profile_pass() {
        let profile = QomProfile::outcome();
        let metrics = QomMetrics::schema_valid().with_tool_outcome_correctness(0.95);
        let eval = profile.evaluate(&metrics);

        assert!(eval.meets_profile);
    }

    #[test]
    fn test_qom_outcome_profile_fail() {
        let profile = QomProfile::outcome();
        let metrics = QomMetrics::schema_valid().with_tool_outcome_correctness(0.85);
        let eval = profile.evaluate(&metrics);

        assert!(!eval.meets_profile);
        assert!(eval
            .failures
            .iter()
            .any(|f| f.metric == "tool_outcome_correctness"));
    }

    #[test]
    fn test_qom_comprehensive_profile() {
        let profile = QomProfile::comprehensive();
        let metrics = QomMetrics::schema_valid()
            .with_instruction_compliance(0.96)
            .with_groundedness(0.85)
            .with_ontology_adherence(0.98)
            .with_tool_outcome_correctness(0.92);

        let eval = profile.evaluate(&metrics);
        assert!(eval.meets_profile);
    }

    #[test]
    fn test_qom_report_pass() {
        let metrics = QomMetrics::schema_valid().with_instruction_compliance(0.99);
        let report = QomReport::pass("qom-strict-argcheck", metrics);

        assert!(report.meets_profile);
        assert_eq!(report.profile, "qom-strict-argcheck");
        assert!(report.hints.is_empty());
    }

    #[test]
    fn test_qom_report_fail() {
        let metrics = QomMetrics::schema_invalid();
        let profile = QomProfile::basic();
        let evaluation = profile.evaluate(&metrics);
        let report = QomReport::fail("qom-basic", metrics, evaluation);

        assert!(!report.meets_profile);
        assert!(!report.hints.is_empty());
    }

    #[test]
    fn test_qom_metrics_builder() {
        let metrics = QomMetrics::schema_valid()
            .with_instruction_compliance(0.95)
            .with_groundedness(0.80)
            .with_ontology_adherence(0.90)
            .with_determinism_jitter(0.85)
            .with_tool_outcome_correctness(0.92);

        assert_eq!(metrics.schema_fidelity, 1.0);
        assert_eq!(metrics.instruction_compliance, Some(0.95));
        assert_eq!(metrics.groundedness, Some(0.80));
        assert_eq!(metrics.ontology_adherence, Some(0.90));
        assert_eq!(metrics.determinism_jitter, Some(0.85));
        assert_eq!(metrics.tool_outcome_correctness, Some(0.92));
    }
}

// =============================================================================
// Section 4: Policy Engine Conformance Tests
// =============================================================================

mod policy_tests {
    use super::*;

    fn test_stype() -> SType {
        SType::parse("eval.rag.RAGQuery.v1").unwrap()
    }

    #[test]
    fn test_policy_pattern_all() {
        let pattern = StypePattern::all();
        assert!(pattern.matches(&test_stype()));
        assert!(pattern.matches(&SType::parse("org.calendar.Event.v1").unwrap()));
    }

    #[test]
    fn test_policy_pattern_namespace() {
        let pattern = StypePattern::namespace("eval");
        assert!(pattern.matches(&test_stype()));
        assert!(!pattern.matches(&SType::parse("org.calendar.Event.v1").unwrap()));
    }

    #[test]
    fn test_policy_pattern_namespace_domain() {
        let pattern = StypePattern::namespace_domain("eval", "rag");
        assert!(pattern.matches(&test_stype()));
        assert!(!pattern.matches(&SType::parse("eval.search.Query.v1").unwrap()));
    }

    #[test]
    fn test_policy_pattern_wildcard() {
        let pattern = StypePattern {
            namespace: Some("ev*".to_string()),
            domain: Some("*ag".to_string()),
            name: None,
            version: None,
        };
        assert!(pattern.matches(&test_stype()));
    }

    #[test]
    fn test_policy_version_constraint_eq() {
        let pattern = StypePattern {
            namespace: Some("eval".to_string()),
            domain: None,
            name: None,
            version: Some(VersionConstraint::Eq { version: 1 }),
        };
        assert!(pattern.matches(&test_stype()));
        assert!(!pattern.matches(&SType::parse("eval.rag.RAGQuery.v2").unwrap()));
    }

    #[test]
    fn test_policy_version_constraint_gte() {
        let pattern = StypePattern {
            namespace: None,
            domain: None,
            name: None,
            version: Some(VersionConstraint::Gte { version: 1 }),
        };
        assert!(pattern.matches(&SType::parse("eval.rag.RAGQuery.v1").unwrap()));
        assert!(pattern.matches(&SType::parse("eval.rag.RAGQuery.v2").unwrap()));
    }

    #[test]
    fn test_policy_version_constraint_range() {
        let pattern = StypePattern {
            namespace: None,
            domain: None,
            name: None,
            version: Some(VersionConstraint::Range { min: 1, max: 3 }),
        };
        assert!(pattern.matches(&SType::parse("eval.test.Type.v1").unwrap()));
        assert!(pattern.matches(&SType::parse("eval.test.Type.v2").unwrap()));
        assert!(pattern.matches(&SType::parse("eval.test.Type.v3").unwrap()));
        assert!(!pattern.matches(&SType::parse("eval.test.Type.v4").unwrap()));
    }

    #[test]
    fn test_policy_engine_qom_override() {
        let mut engine = PolicyEngine::new();
        let policy = Policy::new("eval-strict")
            .with_stype_pattern(StypePattern::namespace("eval"))
            .with_qom_override("qom-strict-argcheck");
        engine.add_policy(policy);

        let context = PolicyContext::new(test_stype(), Operation::Execute);
        let decision = engine.evaluate(&context);

        assert!(decision.is_allowed());
        assert_eq!(
            decision.required_profile,
            Some("qom-strict-argcheck".to_string())
        );
    }

    #[test]
    fn test_policy_access_control_allow() {
        let mut engine = PolicyEngine::new();
        let policy = Policy::new("admin-only")
            .with_stype_pattern(StypePattern::namespace("org"))
            .with_access_control(AccessControlRule {
                allow: HashSet::from(["admin".to_string()]),
                deny: HashSet::new(),
                operation_map: std::collections::HashMap::new(),
                default: AccessDefault::Deny,
            });
        engine.add_policy(policy);

        let stype = SType::parse("org.user.Profile.v1").unwrap();

        // Admin allowed
        let admin_ctx = PolicyContext::new(stype.clone(), Operation::Read).with_principal("admin");
        assert!(engine.evaluate(&admin_ctx).is_allowed());

        // Others denied
        let user_ctx = PolicyContext::new(stype.clone(), Operation::Read).with_principal("user");
        assert!(!engine.evaluate(&user_ctx).is_allowed());

        // Anonymous denied
        let anon_ctx = PolicyContext::new(stype, Operation::Read);
        assert!(!engine.evaluate(&anon_ctx).is_allowed());
    }

    #[test]
    fn test_policy_constraint_has_metadata() {
        let mut engine = PolicyEngine::new();
        let policy = Policy::new("require-tenant")
            .with_stype_pattern(StypePattern::all())
            .with_operations([Operation::Execute]);

        // Add constraint
        let mut policy_with_constraint = policy;
        policy_with_constraint.constraints.push(Constraint {
            name: "tenant-required".to_string(),
            expression: ConstraintExpr::HasMetadata {
                key: "tenant_id".to_string(),
            },
            required: true,
        });
        engine.add_policy(policy_with_constraint);

        // With metadata - allowed
        let ctx_with = PolicyContext::new(test_stype(), Operation::Execute)
            .with_metadata("tenant_id", "acme-corp");
        assert!(engine.evaluate(&ctx_with).is_allowed());

        // Without metadata - denied
        let ctx_without = PolicyContext::new(test_stype(), Operation::Execute);
        assert!(!engine.evaluate(&ctx_without).is_allowed());
    }

    #[test]
    fn test_policy_multiple_policies() {
        let mut engine = PolicyEngine::new();

        // Policy 1: Eval namespace requires strict QoM
        engine.add_policy(
            Policy::new("eval-qom")
                .with_stype_pattern(StypePattern::namespace("eval"))
                .with_qom_override("qom-strict-argcheck"),
        );

        // Policy 2: Org namespace requires basic QoM
        engine.add_policy(
            Policy::new("org-qom")
                .with_stype_pattern(StypePattern::namespace("org"))
                .with_qom_override("qom-basic"),
        );

        // Eval SType
        let eval_ctx = PolicyContext::new(test_stype(), Operation::Execute);
        let eval_decision = engine.evaluate(&eval_ctx);
        assert_eq!(
            eval_decision.required_profile,
            Some("qom-strict-argcheck".to_string())
        );

        // Org SType
        let org_ctx = PolicyContext::new(
            SType::parse("org.calendar.Event.v1").unwrap(),
            Operation::Execute,
        );
        let org_decision = engine.evaluate(&org_ctx);
        assert_eq!(org_decision.required_profile, Some("qom-basic".to_string()));
    }
}

// =============================================================================
// Section 5: Envelope Conformance Tests
// =============================================================================

mod envelope_tests {
    use super::*;

    #[test]
    fn test_envelope_creation() {
        let payload = json!({"title": "Test"});
        let envelope = MplEnvelope::new("org.test.Type.v1".to_string(), payload.clone());

        assert_eq!(envelope.stype, "org.test.Type.v1");
        assert_eq!(envelope.payload, payload);
        assert!(envelope.sem_hash.is_none());
    }

    #[test]
    fn test_envelope_hash_computation() {
        let payload = json!({"title": "Test"});
        let mut envelope = MplEnvelope::new("org.test.Type.v1".to_string(), payload);

        envelope.compute_hash().unwrap();

        assert!(envelope.sem_hash.is_some());
        assert!(envelope.sem_hash.as_ref().unwrap().starts_with("b3:"));
    }

    #[test]
    fn test_envelope_hash_verification() {
        let payload = json!({"title": "Test", "value": 42});
        let mut envelope = MplEnvelope::new("org.test.Type.v1".to_string(), payload);

        envelope.compute_hash().unwrap();

        assert!(envelope.verify_hash().unwrap());
    }

    #[test]
    fn test_envelope_hash_tamper_detection() {
        let payload = json!({"title": "Test"});
        let mut envelope = MplEnvelope::new("org.test.Type.v1".to_string(), payload);

        envelope.compute_hash().unwrap();

        // Tamper with payload
        envelope.payload = json!({"title": "Tampered"});

        // Hash should no longer verify
        assert!(!envelope.verify_hash().unwrap());
    }

    #[test]
    fn test_envelope_serialization() {
        let payload = json!({"data": "test"});
        let mut envelope = MplEnvelope::new("org.test.Type.v1".to_string(), payload);
        envelope.compute_hash().unwrap();

        let json_str = serde_json::to_string(&envelope).unwrap();
        let restored: MplEnvelope = serde_json::from_str(&json_str).unwrap();

        assert_eq!(restored.stype, envelope.stype);
        assert_eq!(restored.payload, envelope.payload);
        assert_eq!(restored.sem_hash, envelope.sem_hash);
    }

    #[test]
    fn test_envelope_with_provenance() {
        let payload = json!({"title": "Test"});
        let envelope =
            MplEnvelope::new("org.test.Type.v1".to_string(), payload).with_profile("qom-basic");

        assert_eq!(envelope.profile, Some("qom-basic".to_string()));
    }
}

// =============================================================================
// Section 6: Hashing Conformance Tests
// =============================================================================

mod hash_tests {
    use super::*;

    #[test]
    fn test_canonicalization_key_ordering() {
        let payload = json!({"z": 1, "a": 2, "m": 3});
        let canonical = canonicalize(&payload).unwrap();

        assert_eq!(canonical, r#"{"a":2,"m":3,"z":1}"#);
    }

    #[test]
    fn test_canonicalization_nested() {
        let payload = json!({
            "outer": {"z": 1, "a": 2},
            "array": [{"b": 1, "a": 2}]
        });
        let canonical = canonicalize(&payload).unwrap();

        // Keys should be sorted at all levels
        assert!(canonical.contains(r#""outer":{"a":2,"z":1}"#));
        assert!(canonical.contains(r#"{"a":2,"b":1}"#));
    }

    #[test]
    fn test_canonicalization_removes_nulls() {
        let payload = json!({"a": 1, "b": null, "c": 3});
        let canonical = canonicalize(&payload).unwrap();

        assert!(!canonical.contains("null"));
        assert_eq!(canonical, r#"{"a":1,"c":3}"#);
    }

    #[test]
    fn test_hash_determinism() {
        let payload = json!({"title": "Test", "value": 42});

        let hash1 = semantic_hash(&payload).unwrap();
        let hash2 = semantic_hash(&payload).unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_different_order_same_result() {
        let p1 = json!({"a": 1, "b": 2});
        let p2 = json!({"b": 2, "a": 1});

        let h1 = semantic_hash(&p1).unwrap();
        let h2 = semantic_hash(&p2).unwrap();

        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_different_values_different_result() {
        let p1 = json!({"value": 1});
        let p2 = json!({"value": 2});

        let h1 = semantic_hash(&p1).unwrap();
        let h2 = semantic_hash(&p2).unwrap();

        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash_verification() {
        let payload = json!({"data": "test"});
        let hash = semantic_hash(&payload).unwrap();

        assert!(verify_hash(&payload, &hash).unwrap());
        assert!(!verify_hash(&json!({"data": "modified"}), &hash).unwrap());
    }

    #[test]
    fn test_hash_format() {
        let payload = json!({"test": true});
        let hash = semantic_hash(&payload).unwrap();

        assert!(hash.starts_with("b3:"));
        // BLAKE3 produces 32 bytes = 64 hex chars
        assert_eq!(hash.len(), 3 + 64); // "b3:" + 64 hex chars
    }
}

// =============================================================================
// Section 7: Handshake Conformance Tests
// =============================================================================

mod handshake_tests {
    use super::*;

    #[test]
    fn test_client_hello_creation() {
        let hello = ClientHello::new()
            .with_stypes(vec![
                "org.calendar.Event.v1".to_string(),
                "org.agent.TaskPlan.v1".to_string(),
            ])
            .with_profile("qom-basic");

        assert_eq!(hello.stypes.len(), 2);
        assert_eq!(hello.profile, Some("qom-basic".to_string()));
    }

    #[test]
    fn test_client_hello_with_features() {
        let hello = ClientHello::new()
            .with_stypes(vec!["org.calendar.Event.v1".to_string()])
            .with_profile("qom-basic")
            .with_features(vec!["compression".to_string(), "streaming".to_string()]);

        assert_eq!(hello.features.len(), 2);
        assert!(hello.features.contains(&"compression".to_string()));
    }

    #[test]
    fn test_server_select_success() {
        let server = ServerSelect::success()
            .with_profile("qom-basic")
            .with_stypes(vec!["org.calendar.Event.v1".to_string()]);

        assert!(server.success);
        assert_eq!(server.profile, Some("qom-basic".to_string()));
        assert!(server.stypes.contains(&"org.calendar.Event.v1".to_string()));
    }

    #[test]
    fn test_server_select_failure() {
        let server = ServerSelect::failed("No common QoM profile");

        assert!(!server.success);
        assert!(server.error.is_some());
        assert!(server.error.unwrap().contains("No common QoM profile"));
    }

    #[test]
    fn test_client_hello_serialization() {
        let hello = ClientHello::new()
            .with_stypes(vec!["org.test.Type.v1".to_string()])
            .with_profile("qom-basic");

        let json_str = serde_json::to_string(&hello).unwrap();
        let restored: ClientHello = serde_json::from_str(&json_str).unwrap();

        assert_eq!(restored.stypes, hello.stypes);
        assert_eq!(restored.profile, hello.profile);
    }

    #[test]
    fn test_server_select_serialization() {
        let server = ServerSelect::success()
            .with_profile("qom-strict-argcheck")
            .with_stypes(vec!["org.calendar.Event.v1".to_string()]);

        let json_str = serde_json::to_string(&server).unwrap();
        let restored: ServerSelect = serde_json::from_str(&json_str).unwrap();

        assert_eq!(restored.profile, server.profile);
        assert_eq!(restored.stypes, server.stypes);
    }
}

// =============================================================================
// Section 8: Error Taxonomy Conformance Tests
// =============================================================================

mod error_tests {
    use super::*;
    use mpl_core::error::SchemaError;

    #[test]
    fn test_error_schema_fidelity() {
        let error = MplError::SchemaFidelity {
            message: "Validation failed".to_string(),
            stype: "org.test.Type.v1".to_string(),
            errors: vec![SchemaError {
                path: "$.name".to_string(),
                message: "missing required field".to_string(),
                expected: Some("string".to_string()),
                actual: None,
            }],
            hints: vec!["Add the 'name' field".to_string()],
        };
        assert_eq!(error.code(), MplErrorCode::ESchemaFidelity);
    }

    #[test]
    fn test_error_qom_breach() {
        let error = MplError::QomBreach {
            message: "QoM threshold not met".to_string(),
            metrics: std::collections::HashMap::from([(
                "instruction_compliance".to_string(),
                0.85,
            )]),
            thresholds: std::collections::HashMap::from([(
                "instruction_compliance".to_string(),
                0.97,
            )]),
            hints: vec!["Improve instruction compliance".to_string()],
        };
        assert_eq!(error.code(), MplErrorCode::EQomBreach);
    }

    #[test]
    fn test_error_unknown_stype() {
        let error = MplError::UnknownStype {
            stype: "unknown.Type.v99".to_string(),
            suggestions: vec!["unknown.Type.v1".to_string()],
        };
        assert_eq!(error.code(), MplErrorCode::EUnknownStype);
    }

    #[test]
    fn test_error_code_display() {
        assert_eq!(MplErrorCode::ESchemaFidelity.as_str(), "E-SCHEMA-FIDELITY");
        assert_eq!(MplErrorCode::EQomBreach.as_str(), "E-QOM-BREACH");
        assert_eq!(MplErrorCode::EUnknownStype.as_str(), "E-UNKNOWN-STYPE");
        assert_eq!(MplErrorCode::EPolicyDenied.as_str(), "E-POLICY-DENIED");
    }

    #[test]
    fn test_error_display() {
        let error = MplError::SchemaFidelity {
            message: "Validation failed for org.test.Type.v1".to_string(),
            stype: "org.test.Type.v1".to_string(),
            errors: vec![],
            hints: vec![],
        };

        let display = format!("{}", error);
        assert!(display.contains("Schema validation failed"));
    }
}

// =============================================================================
// Section 9: Integration Pipeline Tests
// =============================================================================

mod integration_tests {
    use super::*;

    #[test]
    fn test_full_validation_pipeline() {
        // 1. Parse SType
        let stype = SType::parse("eval.rag.RAGQuery.v1").unwrap();

        // 2. Setup validator
        let mut validator = SchemaValidator::new();
        validator
            .register(
                &stype.id(),
                json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"},
                        "top_k": {"type": "integer", "minimum": 1}
                    },
                    "required": ["query"]
                }),
            )
            .unwrap();

        // 3. Create and validate payload
        let payload = json!({
            "query": "What is MPL?",
            "top_k": 5
        });
        let result = validator.validate(&stype.id(), &payload).unwrap();
        assert!(result.valid);

        // 4. Create envelope with hash
        let mut envelope = MplEnvelope::new(stype.id(), payload);
        envelope.compute_hash().unwrap();
        assert!(envelope.verify_hash().unwrap());

        // 5. Evaluate QoM
        let profile = QomProfile::basic();
        let metrics = result.to_qom_metrics();
        let evaluation = profile.evaluate(&metrics);
        assert!(evaluation.meets_profile);

        // 6. Policy check
        let mut engine = PolicyEngine::new();
        engine.add_policy(
            Policy::new("eval-policy")
                .with_stype_pattern(StypePattern::namespace("eval"))
                .with_qom_override("qom-basic"),
        );
        let context = PolicyContext::new(stype, Operation::Execute);
        let decision = engine.evaluate(&context);
        assert!(decision.is_allowed());
    }

    #[test]
    fn test_validation_failure_pipeline() {
        let stype = SType::parse("eval.test.Type.v1").unwrap();

        let mut validator = SchemaValidator::new();
        validator
            .register(
                &stype.id(),
                json!({
                    "type": "object",
                    "properties": {
                        "required_field": {"type": "string"}
                    },
                    "required": ["required_field"]
                }),
            )
            .unwrap();

        // Invalid payload
        let payload = json!({"other_field": "value"});
        let result = validator.validate(&stype.id(), &payload).unwrap();
        assert!(!result.valid);

        // QoM should fail
        let metrics = result.to_qom_metrics();
        assert_eq!(metrics.schema_fidelity, 0.0);

        let profile = QomProfile::basic();
        let evaluation = profile.evaluate(&metrics);
        assert!(!evaluation.meets_profile);
    }

    #[test]
    fn test_handshake_to_validation_pipeline() {
        // 1. Client hello
        let _client_hello = ClientHello::new()
            .with_stypes(vec!["org.calendar.Event.v1".to_string()])
            .with_profile("qom-basic");

        // 2. Server select (successful negotiation)
        let server_select = ServerSelect::success()
            .with_stypes(vec!["org.calendar.Event.v1".to_string()])
            .with_profile("qom-basic");

        // 3. Validate SType is agreed
        assert!(server_select
            .stypes
            .contains(&"org.calendar.Event.v1".to_string()));

        // 4. Setup validation for agreed STypes
        let mut validator = SchemaValidator::new();
        for stype_id in &server_select.stypes {
            validator
                .register(
                    stype_id,
                    json!({
                        "type": "object",
                        "properties": {
                            "title": {"type": "string"}
                        },
                        "required": ["title"]
                    }),
                )
                .unwrap();
        }

        // 5. Use agreed profile for QoM
        let profile = match server_select.profile.as_deref() {
            Some("qom-basic") => QomProfile::basic(),
            Some("qom-strict-argcheck") => QomProfile::strict_argcheck(),
            _ => QomProfile::basic(),
        };

        // 6. Validate payload
        let payload = json!({"title": "Meeting"});
        let result = validator
            .validate("org.calendar.Event.v1", &payload)
            .unwrap();
        let metrics = result.to_qom_metrics();
        let evaluation = profile.evaluate(&metrics);

        assert!(evaluation.meets_profile);
    }

    #[test]
    fn test_policy_with_qom_pipeline() {
        // Setup policy that requires strict QoM for certain namespaces
        // Policies are evaluated in order, so more specific should come first
        let mut engine = PolicyEngine::new();

        // Add specific policy first (it will be evaluated but both match)
        engine.add_policy(
            Policy::new("financial-strict")
                .with_stype_pattern(StypePattern::namespace_domain("org", "finance"))
                .with_qom_override("qom-comprehensive"),
        );

        // Financial SType should require comprehensive QoM
        let financial_stype = SType::parse("org.finance.Transaction.v1").unwrap();
        let financial_ctx = PolicyContext::new(financial_stype, Operation::Execute);
        let financial_decision = engine.evaluate(&financial_ctx);
        assert_eq!(
            financial_decision.required_profile,
            Some("qom-comprehensive".to_string())
        );

        // For calendar, add a specific policy
        let mut engine2 = PolicyEngine::new();
        engine2.add_policy(
            Policy::new("calendar-basic")
                .with_stype_pattern(StypePattern::namespace_domain("org", "calendar"))
                .with_qom_override("qom-basic"),
        );

        let calendar_stype = SType::parse("org.calendar.Event.v1").unwrap();
        let calendar_ctx = PolicyContext::new(calendar_stype, Operation::Execute);
        let calendar_decision = engine2.evaluate(&calendar_ctx);
        assert_eq!(
            calendar_decision.required_profile,
            Some("qom-basic".to_string())
        );
    }
}
