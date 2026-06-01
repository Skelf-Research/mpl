//! Schema Validation
//!
//! JSON Schema validation for SType payloads.
//! Schema Fidelity is the mandatory QoM metric.

use jsonschema::{Draft, JSONSchema};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{MplError, Result, SchemaError};
use crate::qom::QomMetrics;

/// Default maximum payload size (1 MB)
pub const DEFAULT_MAX_PAYLOAD_SIZE: usize = 1024 * 1024;

/// Default maximum schema cache size
pub const DEFAULT_MAX_SCHEMAS: usize = 1000;

/// Default maximum nesting depth for JSON payloads
pub const DEFAULT_MAX_NESTING_DEPTH: usize = 50;

/// Schema validator with caching for performance
pub struct SchemaValidator {
    /// Cached compiled schemas
    schemas: HashMap<String, Arc<JSONSchema>>,
    /// Maximum payload size in bytes
    max_payload_size: usize,
    /// Maximum number of cached schemas
    max_schemas: usize,
}

impl SchemaValidator {
    /// Create a new validator with default limits
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
            max_payload_size: DEFAULT_MAX_PAYLOAD_SIZE,
            max_schemas: DEFAULT_MAX_SCHEMAS,
        }
    }

    /// Create a validator with custom limits
    pub fn with_limits(max_payload_size: usize, max_schemas: usize) -> Self {
        Self {
            schemas: HashMap::new(),
            max_payload_size,
            max_schemas,
        }
    }

    /// Register a schema for an SType
    pub fn register(&mut self, stype: &str, schema: Value) -> Result<()> {
        // Enforce schema cache limit (skip if already registered)
        if !self.schemas.contains_key(stype) && self.schemas.len() >= self.max_schemas {
            return Err(MplError::Validation(format!(
                "Schema cache limit reached ({}). Cannot register schema for {}",
                self.max_schemas, stype
            )));
        }

        let compiled = JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(&schema)
            .map_err(|e| MplError::Validation(format!("Invalid schema for {}: {}", stype, e)))?;

        self.schemas.insert(stype.to_string(), Arc::new(compiled));
        Ok(())
    }

    /// Get current number of registered schemas
    pub fn schema_count(&self) -> usize {
        self.schemas.len()
    }

    /// Get maximum payload size limit
    pub fn max_payload_size(&self) -> usize {
        self.max_payload_size
    }

    /// Register a schema from a JSON string
    pub fn register_json(&mut self, stype: &str, schema_json: &str) -> Result<()> {
        let schema: Value = serde_json::from_str(schema_json)?;
        self.register(stype, schema)
    }

    /// Check if a schema is registered
    pub fn has_schema(&self, stype: &str) -> bool {
        self.schemas.contains_key(stype)
    }

    /// Validate a payload against its declared SType
    pub fn validate(&self, stype: &str, payload: &Value) -> Result<ValidationResult> {
        // Check payload size (estimate based on JSON serialization)
        let payload_size = estimate_json_size(payload);
        if payload_size > self.max_payload_size {
            return Err(MplError::Validation(format!(
                "Payload size ({} bytes) exceeds maximum ({} bytes) for SType {}",
                payload_size, self.max_payload_size, stype
            )));
        }

        let schema = self.schemas.get(stype).ok_or_else(|| MplError::UnknownStype {
            stype: stype.to_string(),
            suggestions: self.suggest_similar(stype),
        })?;

        let result = schema.validate(payload);

        match result {
            Ok(_) => Ok(ValidationResult::valid()),
            Err(errors) => {
                let schema_errors: Vec<SchemaError> = errors
                    .map(|e| SchemaError {
                        path: e.instance_path.to_string(),
                        message: e.to_string(),
                        expected: None,
                        actual: None,
                    })
                    .collect();

                Ok(ValidationResult::invalid(schema_errors))
            }
        }
    }

    /// Validate and return QoM metrics
    pub fn validate_qom(&self, stype: &str, payload: &Value) -> Result<QomMetrics> {
        let result = self.validate(stype, payload)?;
        Ok(QomMetrics {
            schema_fidelity: if result.valid { 1.0 } else { 0.0 },
            ..Default::default()
        })
    }

    /// Validate and return an MplError if invalid
    pub fn validate_or_error(&self, stype: &str, payload: &Value) -> Result<()> {
        let result = self.validate(stype, payload)?;

        if result.valid {
            Ok(())
        } else {
            Err(MplError::SchemaFidelity {
                message: format!("Payload does not conform to {}", stype),
                stype: stype.to_string(),
                errors: result.errors,
                hints: vec![
                    "Check required fields are present".to_string(),
                    "Verify field types match schema".to_string(),
                ],
            })
        }
    }

    /// Suggest similar STypes for typo correction
    fn suggest_similar(&self, stype: &str) -> Vec<String> {
        self.schemas
            .keys()
            .filter(|k| {
                // Simple similarity: same suffix or prefix
                k.ends_with(stype.split('.').last().unwrap_or(""))
                    || k.starts_with(stype.split('.').next().unwrap_or(""))
            })
            .take(3)
            .cloned()
            .collect()
    }

    /// Get all registered STypes
    pub fn registered_stypes(&self) -> Vec<&str> {
        self.schemas.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for SchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of schema validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation passed
    pub valid: bool,
    /// Validation errors (empty if valid)
    pub errors: Vec<SchemaError>,
}

impl ValidationResult {
    /// Create a valid result
    pub fn valid() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
        }
    }

    /// Create an invalid result
    pub fn invalid(errors: Vec<SchemaError>) -> Self {
        Self {
            valid: false,
            errors,
        }
    }

    /// Convert to QoM metrics
    pub fn to_qom_metrics(&self) -> QomMetrics {
        QomMetrics {
            schema_fidelity: if self.valid { 1.0 } else { 0.0 },
            ..Default::default()
        }
    }
}

/// Builder for creating validators with common schemas
pub struct ValidatorBuilder {
    validator: SchemaValidator,
}

impl ValidatorBuilder {
    pub fn new() -> Self {
        Self {
            validator: SchemaValidator::new(),
        }
    }

    /// Add a schema
    pub fn with_schema(mut self, stype: &str, schema: Value) -> Result<Self> {
        self.validator.register(stype, schema)?;
        Ok(self)
    }

    /// Add a schema from JSON string
    pub fn with_schema_json(mut self, stype: &str, schema_json: &str) -> Result<Self> {
        self.validator.register_json(stype, schema_json)?;
        Ok(self)
    }

    /// Build the validator
    pub fn build(self) -> SchemaValidator {
        self.validator
    }
}

impl Default for ValidatorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Estimate the size of a JSON value in bytes (rough approximation)
fn estimate_json_size(value: &Value) -> usize {
    match value {
        Value::Null => 4, // "null"
        Value::Bool(b) => if *b { 4 } else { 5 }, // "true" or "false"
        Value::Number(n) => n.to_string().len(),
        Value::String(s) => s.len() + 2, // quotes
        Value::Array(arr) => {
            arr.iter().map(estimate_json_size).sum::<usize>() + arr.len() + 2 // commas + brackets
        }
        Value::Object(obj) => {
            obj.iter()
                .map(|(k, v)| k.len() + 3 + estimate_json_size(v)) // key + quotes + colon
                .sum::<usize>()
                + obj.len()
                + 2 // commas + braces
        }
    }
}

/// Check JSON nesting depth (to prevent stack overflow attacks)
pub fn check_nesting_depth(value: &Value, max_depth: usize) -> Result<()> {
    fn check_depth(value: &Value, current: usize, max: usize) -> bool {
        if current > max {
            return false;
        }
        match value {
            Value::Array(arr) => arr.iter().all(|v| check_depth(v, current + 1, max)),
            Value::Object(obj) => obj.values().all(|v| check_depth(v, current + 1, max)),
            _ => true,
        }
    }

    if check_depth(value, 0, max_depth) {
        Ok(())
    } else {
        Err(MplError::Validation(format!(
            "JSON nesting depth exceeds maximum of {}",
            max_depth
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_schema() -> Value {
        json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "title": {"type": "string"},
                "start": {"type": "string", "format": "date-time"},
                "end": {"type": "string", "format": "date-time"}
            },
            "required": ["title", "start", "end"],
            "additionalProperties": false
        })
    }

    #[test]
    fn test_register_and_validate() {
        let mut validator = SchemaValidator::new();
        validator
            .register("org.calendar.Event.v1", sample_schema())
            .unwrap();

        let valid_payload = json!({
            "title": "Meeting",
            "start": "2025-01-01T10:00:00Z",
            "end": "2025-01-01T11:00:00Z"
        });

        let result = validator
            .validate("org.calendar.Event.v1", &valid_payload)
            .unwrap();
        assert!(result.valid);
    }

    #[test]
    fn test_invalid_payload() {
        let mut validator = SchemaValidator::new();
        validator
            .register("org.calendar.Event.v1", sample_schema())
            .unwrap();

        let invalid_payload = json!({
            "title": "Meeting"
            // missing start and end
        });

        let result = validator
            .validate("org.calendar.Event.v1", &invalid_payload)
            .unwrap();
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_unknown_stype() {
        let validator = SchemaValidator::new();
        let result = validator.validate("unknown.Type.v1", &json!({}));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MplError::UnknownStype { .. }));
    }

    #[test]
    fn test_qom_metrics() {
        let mut validator = SchemaValidator::new();
        validator
            .register("org.test.Test.v1", json!({"type": "object"}))
            .unwrap();

        let metrics = validator
            .validate_qom("org.test.Test.v1", &json!({}))
            .unwrap();
        assert_eq!(metrics.schema_fidelity, 1.0);
    }

    #[test]
    fn test_builder() {
        let validator = ValidatorBuilder::new()
            .with_schema("org.test.Test.v1", json!({"type": "object"}))
            .unwrap()
            .build();

        assert!(validator.has_schema("org.test.Test.v1"));
    }
}
