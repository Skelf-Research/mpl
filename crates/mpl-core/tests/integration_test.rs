//! Integration tests for MPL core library

use mpl_core::envelope::MplEnvelope;
use mpl_core::hash::{canonicalize, semantic_hash, verify_hash};
use mpl_core::qom::{QomMetrics, QomProfile};
use mpl_core::stype::SType;
use mpl_core::validation::SchemaValidator;
use serde_json::json;

/// Test the full validation pipeline
#[test]
fn test_full_validation_pipeline() {
    // 1. Parse SType
    let stype = SType::parse("org.calendar.Event.v1").unwrap();
    assert_eq!(stype.name, "Event");

    // 2. Create schema validator
    let mut validator = SchemaValidator::new();
    let schema = json!({
        "type": "object",
        "properties": {
            "title": {"type": "string"},
            "start": {"type": "string"},
            "end": {"type": "string"}
        },
        "required": ["title", "start", "end"]
    });
    validator.register(&stype.id(), schema).unwrap();

    // 3. Create payload
    let payload = json!({
        "title": "Team Meeting",
        "start": "2024-01-15T10:00:00Z",
        "end": "2024-01-15T11:00:00Z"
    });

    // 4. Validate
    let result = validator.validate(&stype.id(), &payload).unwrap();
    assert!(result.valid);

    // 5. Compute semantic hash
    let hash = semantic_hash(&payload).unwrap();
    assert!(hash.starts_with("b3:"));

    // 6. Verify hash
    assert!(verify_hash(&payload, &hash).unwrap());

    // 7. QoM evaluation
    let profile = QomProfile::basic();
    let metrics = QomMetrics::schema_valid();
    let evaluation = profile.evaluate(&metrics);
    assert!(evaluation.meets_profile);
}

/// Test validation failure
#[test]
fn test_validation_failure_pipeline() {
    let mut validator = SchemaValidator::new();
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"}
        },
        "required": ["name"]
    });
    validator.register("test.Type.v1", schema).unwrap();

    // Invalid payload - missing required field
    let payload = json!({"other": "value"});
    let result = validator.validate("test.Type.v1", &payload).unwrap();

    assert!(!result.valid);
    assert!(!result.errors.is_empty());

    // QoM should fail
    let profile = QomProfile::basic();
    let metrics = QomMetrics::schema_invalid();
    let evaluation = profile.evaluate(&metrics);
    assert!(!evaluation.meets_profile);
}

/// Test envelope creation and serialization
#[test]
fn test_envelope_round_trip() {
    let payload = json!({
        "title": "Meeting",
        "start": "2024-01-15T10:00:00Z",
        "end": "2024-01-15T11:00:00Z"
    });

    let mut envelope = MplEnvelope::new("org.calendar.Event.v1".to_string(), payload.clone());

    // Compute hash
    envelope.compute_hash().unwrap();
    assert!(envelope.sem_hash.is_some());

    // Serialize
    let json_str = serde_json::to_string(&envelope).unwrap();

    // Deserialize
    let restored: MplEnvelope = serde_json::from_str(&json_str).unwrap();

    assert_eq!(restored.stype, envelope.stype);
    assert_eq!(restored.payload, envelope.payload);
    assert_eq!(restored.sem_hash, envelope.sem_hash);

    // Verify hash still valid
    assert!(restored.verify_hash().unwrap());
}

/// Test canonical form is deterministic
#[test]
fn test_canonical_determinism() {
    let payloads = vec![
        json!({"z": 1, "a": 2, "m": 3}),
        json!({"a": 2, "z": 1, "m": 3}),
        json!({"m": 3, "z": 1, "a": 2}),
    ];

    let canonical_forms: Vec<String> = payloads
        .iter()
        .map(|p| canonicalize(p).unwrap())
        .collect();

    // All should be the same
    assert!(canonical_forms.windows(2).all(|w| w[0] == w[1]));

    // And should be sorted
    assert_eq!(canonical_forms[0], r#"{"a":2,"m":3,"z":1}"#);
}

/// Test strict QoM profile
#[test]
fn test_strict_qom_profile() {
    let profile = QomProfile::strict_argcheck();

    // Pass case - strict requires >= 0.97 for instruction_compliance
    let pass_metrics = QomMetrics::schema_valid().with_instruction_compliance(0.98);
    let pass_eval = profile.evaluate(&pass_metrics);
    assert!(pass_eval.meets_profile);

    // Fail case - low instruction compliance
    let fail_metrics = QomMetrics::schema_valid().with_instruction_compliance(0.5);
    let fail_eval = profile.evaluate(&fail_metrics);
    assert!(!fail_eval.meets_profile);
    assert!(fail_eval.failures.iter().any(|f| f.metric == "instruction_compliance"));
}

/// Test multiple schema registration
#[test]
fn test_multiple_schemas() {
    let mut validator = SchemaValidator::new();

    // Register multiple schemas
    validator
        .register(
            "org.calendar.Event.v1",
            json!({
                "type": "object",
                "properties": {"title": {"type": "string"}},
                "required": ["title"]
            }),
        )
        .unwrap();

    validator
        .register(
            "org.agent.TaskPlan.v1",
            json!({
                "type": "object",
                "properties": {"goal": {"type": "string"}},
                "required": ["goal"]
            }),
        )
        .unwrap();

    // Validate against correct schemas
    let event = json!({"title": "Meeting"});
    let task = json!({"goal": "Complete project"});

    assert!(validator.validate("org.calendar.Event.v1", &event).unwrap().valid);
    assert!(validator.validate("org.agent.TaskPlan.v1", &task).unwrap().valid);

    // Cross-validation should fail
    assert!(!validator.validate("org.calendar.Event.v1", &task).unwrap().valid);
    assert!(!validator.validate("org.agent.TaskPlan.v1", &event).unwrap().valid);
}

/// Test SType parsing edge cases
#[test]
fn test_stype_parsing() {
    // Simple
    let s1 = SType::parse("org.calendar.Event.v1").unwrap();
    assert_eq!(s1.namespace, "org");
    assert_eq!(s1.domain, "calendar");
    assert_eq!(s1.name, "Event");
    assert_eq!(s1.major_version, 1);

    // Nested namespace
    let s2 = SType::parse("com.acme.finance.Report.v2").unwrap();
    assert_eq!(s2.namespace, "com.acme");
    assert_eq!(s2.domain, "finance");
    assert_eq!(s2.name, "Report");
    assert_eq!(s2.major_version, 2);

    // URN format
    let s3 = SType::parse("urn:stype:org.test.Type.v1").unwrap();
    assert_eq!(s3.id(), "org.test.Type.v1");
}
