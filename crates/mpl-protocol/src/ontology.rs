//! Ontology Adherence Checking
//!
//! Verifies that responses conform to domain-specific ontology constraints
//! beyond what JSON Schema can express.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::debug;

/// Ontology definition for a domain
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Ontology {
    /// Name of this ontology
    #[serde(default)]
    pub name: String,

    /// Description
    #[serde(default)]
    pub description: String,

    /// Allowed values for specific fields (enum constraints)
    #[serde(default)]
    pub allowed_values: HashMap<String, Vec<serde_json::Value>>,

    /// Required field relationships
    #[serde(default)]
    pub relationships: Vec<Relationship>,

    /// Field type constraints (more specific than JSON Schema)
    #[serde(default)]
    pub type_constraints: HashMap<String, TypeConstraint>,

    /// Cardinality constraints
    #[serde(default)]
    pub cardinality: HashMap<String, CardinalityConstraint>,

    /// Custom validation rules (CEL expressions)
    #[serde(default)]
    pub custom_rules: Vec<CustomRule>,
}

/// Relationship between fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// Unique identifier
    pub id: String,

    /// Source field path
    pub from: String,

    /// Target field path
    pub to: String,

    /// Type of relationship
    pub relation_type: RelationType,

    /// Optional condition (when this relationship applies)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,

    /// Error message on violation
    #[serde(default)]
    pub message: String,
}

/// Types of relationships
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    /// from implies to must exist
    Implies,
    /// from and to are mutually exclusive
    Excludes,
    /// from must be less than to
    LessThan,
    /// from must be less than or equal to to
    LessThanOrEqual,
    /// from must be greater than to
    GreaterThan,
    /// from must be greater than or equal to to
    GreaterThanOrEqual,
    /// from must equal to
    Equals,
    /// from must not equal to
    NotEquals,
    /// from must be a subset of to (for arrays)
    SubsetOf,
    /// from must contain to
    Contains,
    /// from must start with to (for strings)
    StartsWith,
    /// from must end with to (for strings)
    EndsWith,
}

/// Type constraint for a field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeConstraint {
    /// Expected semantic type (e.g., "email", "url", "phone", "uuid")
    pub semantic_type: String,

    /// Optional regex pattern
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,

    /// Optional min value (for numbers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,

    /// Optional max value (for numbers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,

    /// Optional min length (for strings/arrays)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,

    /// Optional max length (for strings/arrays)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
}

/// Cardinality constraint for arrays
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardinalityConstraint {
    /// Minimum items
    #[serde(default)]
    pub min: usize,

    /// Maximum items (None = unlimited)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<usize>,

    /// Whether items must be unique
    #[serde(default)]
    pub unique: bool,
}

/// Custom validation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRule {
    /// Unique identifier
    pub id: String,

    /// CEL expression that must evaluate to true
    pub expression: String,

    /// Error message on violation
    pub message: String,

    /// Severity level
    #[serde(default)]
    pub severity: ViolationSeverity,
}

/// Severity of ontology violations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViolationSeverity {
    /// Critical violation - must be fixed
    #[default]
    Error,
    /// Non-critical but should be addressed
    Warning,
    /// Informational only
    Info,
}

/// A single ontology violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyViolation {
    /// Rule or constraint ID that was violated
    pub rule_id: String,

    /// Type of violation
    pub violation_type: ViolationType,

    /// Field path involved
    pub path: String,

    /// Error message
    pub message: String,

    /// Severity
    pub severity: ViolationSeverity,

    /// Expected value (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<serde_json::Value>,

    /// Actual value (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<serde_json::Value>,
}

/// Types of violations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViolationType {
    /// Value not in allowed set
    InvalidValue,
    /// Relationship constraint violated
    RelationshipViolation,
    /// Type constraint violated
    TypeViolation,
    /// Cardinality constraint violated
    CardinalityViolation,
    /// Custom rule violated
    CustomRuleViolation,
}

/// Result of ontology adherence check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyResult {
    /// Overall adherence score (0.0 - 1.0)
    pub score: f64,

    /// Whether the payload adheres to the ontology
    pub adheres: bool,

    /// Total constraints checked
    pub constraints_checked: usize,

    /// Number of violations
    pub violation_count: usize,

    /// Error-level violations
    pub error_count: usize,

    /// Warning-level violations
    pub warning_count: usize,

    /// All violations found
    pub violations: Vec<OntologyViolation>,
}

/// Ontology adherence checker
pub struct OntologyChecker {
    ontology: Ontology,
}

impl OntologyChecker {
    /// Create a new checker with the given ontology
    pub fn new(ontology: Ontology) -> Self {
        Self { ontology }
    }

    /// Check a payload against the ontology
    pub fn check(&self, payload: &serde_json::Value) -> OntologyResult {
        let mut violations = Vec::new();
        let mut constraints_checked = 0;

        // Check allowed values
        for (path, allowed) in &self.ontology.allowed_values {
            constraints_checked += 1;
            if let Some(value) = get_json_path(payload, path) {
                if !allowed.contains(value) {
                    violations.push(OntologyViolation {
                        rule_id: format!("allowed_values:{}", path),
                        violation_type: ViolationType::InvalidValue,
                        path: path.clone(),
                        message: format!("Value '{}' not in allowed set", value),
                        severity: ViolationSeverity::Error,
                        expected: Some(serde_json::Value::Array(allowed.clone())),
                        actual: Some(value.clone()),
                    });
                }
            }
        }

        // Check relationships
        for rel in &self.ontology.relationships {
            constraints_checked += 1;
            if let Some(violation) = self.check_relationship(payload, rel) {
                violations.push(violation);
            }
        }

        // Check type constraints
        for (path, constraint) in &self.ontology.type_constraints {
            constraints_checked += 1;
            if let Some(value) = get_json_path(payload, path) {
                if let Some(violation) = self.check_type_constraint(path, value, constraint) {
                    violations.push(violation);
                }
            }
        }

        // Check cardinality
        for (path, constraint) in &self.ontology.cardinality {
            constraints_checked += 1;
            if let Some(value) = get_json_path(payload, path) {
                if let Some(violation) = self.check_cardinality(path, value, constraint) {
                    violations.push(violation);
                }
            }
        }

        // Count by severity
        let error_count = violations
            .iter()
            .filter(|v| v.severity == ViolationSeverity::Error)
            .count();
        let warning_count = violations
            .iter()
            .filter(|v| v.severity == ViolationSeverity::Warning)
            .count();

        let score = if constraints_checked > 0 {
            1.0 - (violations.len() as f64 / constraints_checked as f64)
        } else {
            1.0
        };

        let adheres = error_count == 0;

        debug!(
            "Ontology check: score={:.2}, violations={}, adheres={}",
            score,
            violations.len(),
            adheres
        );

        OntologyResult {
            score,
            adheres,
            constraints_checked,
            violation_count: violations.len(),
            error_count,
            warning_count,
            violations,
        }
    }

    /// Check a relationship constraint
    fn check_relationship(
        &self,
        payload: &serde_json::Value,
        rel: &Relationship,
    ) -> Option<OntologyViolation> {
        let from_value = get_json_path(payload, &rel.from);
        let to_value = get_json_path(payload, &rel.to);

        let violated = match rel.relation_type {
            RelationType::Implies => {
                // If from exists and is truthy, to must exist
                from_value.map(is_truthy).unwrap_or(false) && to_value.is_none()
            }
            RelationType::Excludes => {
                // from and to cannot both exist
                from_value.is_some() && to_value.is_some()
            }
            RelationType::LessThan => {
                match (
                    from_value.and_then(|v| v.as_f64()),
                    to_value.and_then(|v| v.as_f64()),
                ) {
                    (Some(f), Some(t)) => f >= t,
                    _ => false,
                }
            }
            RelationType::LessThanOrEqual => {
                match (
                    from_value.and_then(|v| v.as_f64()),
                    to_value.and_then(|v| v.as_f64()),
                ) {
                    (Some(f), Some(t)) => f > t,
                    _ => false,
                }
            }
            RelationType::GreaterThan => {
                match (
                    from_value.and_then(|v| v.as_f64()),
                    to_value.and_then(|v| v.as_f64()),
                ) {
                    (Some(f), Some(t)) => f <= t,
                    _ => false,
                }
            }
            RelationType::GreaterThanOrEqual => {
                match (
                    from_value.and_then(|v| v.as_f64()),
                    to_value.and_then(|v| v.as_f64()),
                ) {
                    (Some(f), Some(t)) => f < t,
                    _ => false,
                }
            }
            RelationType::Equals => from_value != to_value,
            RelationType::NotEquals => from_value == to_value && from_value.is_some(),
            RelationType::SubsetOf => match (from_value, to_value) {
                (Some(serde_json::Value::Array(from)), Some(serde_json::Value::Array(to))) => {
                    let to_set: HashSet<_> = to.iter().collect();
                    !from.iter().all(|v| to_set.contains(v))
                }
                _ => false,
            },
            RelationType::Contains => match (from_value, to_value) {
                (Some(serde_json::Value::String(s)), Some(serde_json::Value::String(sub))) => {
                    !s.contains(sub.as_str())
                }
                (Some(serde_json::Value::Array(arr)), Some(item)) => !arr.contains(item),
                _ => false,
            },
            RelationType::StartsWith => match (from_value, to_value) {
                (Some(serde_json::Value::String(s)), Some(serde_json::Value::String(prefix))) => {
                    !s.starts_with(prefix.as_str())
                }
                _ => false,
            },
            RelationType::EndsWith => match (from_value, to_value) {
                (Some(serde_json::Value::String(s)), Some(serde_json::Value::String(suffix))) => {
                    !s.ends_with(suffix.as_str())
                }
                _ => false,
            },
        };

        if violated {
            Some(OntologyViolation {
                rule_id: rel.id.clone(),
                violation_type: ViolationType::RelationshipViolation,
                path: rel.from.clone(),
                message: if rel.message.is_empty() {
                    format!(
                        "Relationship {:?} between '{}' and '{}' violated",
                        rel.relation_type, rel.from, rel.to
                    )
                } else {
                    rel.message.clone()
                },
                severity: ViolationSeverity::Error,
                expected: None,
                actual: from_value.cloned(),
            })
        } else {
            None
        }
    }

    /// Check a type constraint
    fn check_type_constraint(
        &self,
        path: &str,
        value: &serde_json::Value,
        constraint: &TypeConstraint,
    ) -> Option<OntologyViolation> {
        // Check semantic type patterns
        let valid = match constraint.semantic_type.as_str() {
            "email" => value
                .as_str()
                .map(|s| s.contains('@') && s.contains('.'))
                .unwrap_or(false),
            "url" => value
                .as_str()
                .map(|s| s.starts_with("http://") || s.starts_with("https://"))
                .unwrap_or(false),
            "uuid" => value
                .as_str()
                .map(|s| s.len() == 36 && s.chars().filter(|c| *c == '-').count() == 4)
                .unwrap_or(false),
            "phone" => value
                .as_str()
                .map(|s| s.chars().filter(|c| c.is_ascii_digit()).count() >= 10)
                .unwrap_or(false),
            "date" => value
                .as_str()
                .map(|s| s.len() == 10 && s.chars().filter(|c| *c == '-').count() == 2)
                .unwrap_or(false),
            "datetime" => value
                .as_str()
                .map(|s| s.contains('T') || s.contains(' '))
                .unwrap_or(false),
            _ => true, // Unknown type, skip
        };

        if !valid {
            return Some(OntologyViolation {
                rule_id: format!("type:{}", path),
                violation_type: ViolationType::TypeViolation,
                path: path.to_string(),
                message: format!(
                    "Value does not match semantic type '{}'",
                    constraint.semantic_type
                ),
                severity: ViolationSeverity::Error,
                expected: Some(serde_json::Value::String(constraint.semantic_type.clone())),
                actual: Some(value.clone()),
            });
        }

        // Check numeric range
        if let Some(num) = value.as_f64() {
            if let Some(min) = constraint.min {
                if num < min {
                    return Some(OntologyViolation {
                        rule_id: format!("type:{}", path),
                        violation_type: ViolationType::TypeViolation,
                        path: path.to_string(),
                        message: format!("Value {} is less than minimum {}", num, min),
                        severity: ViolationSeverity::Error,
                        expected: Some(serde_json::json!({"min": min})),
                        actual: Some(value.clone()),
                    });
                }
            }
            if let Some(max) = constraint.max {
                if num > max {
                    return Some(OntologyViolation {
                        rule_id: format!("type:{}", path),
                        violation_type: ViolationType::TypeViolation,
                        path: path.to_string(),
                        message: format!("Value {} is greater than maximum {}", num, max),
                        severity: ViolationSeverity::Error,
                        expected: Some(serde_json::json!({"max": max})),
                        actual: Some(value.clone()),
                    });
                }
            }
        }

        // Check string length
        if let Some(s) = value.as_str() {
            if let Some(min_len) = constraint.min_length {
                if s.len() < min_len {
                    return Some(OntologyViolation {
                        rule_id: format!("type:{}", path),
                        violation_type: ViolationType::TypeViolation,
                        path: path.to_string(),
                        message: format!(
                            "String length {} is less than minimum {}",
                            s.len(),
                            min_len
                        ),
                        severity: ViolationSeverity::Error,
                        expected: Some(serde_json::json!({"min_length": min_len})),
                        actual: Some(value.clone()),
                    });
                }
            }
            if let Some(max_len) = constraint.max_length {
                if s.len() > max_len {
                    return Some(OntologyViolation {
                        rule_id: format!("type:{}", path),
                        violation_type: ViolationType::TypeViolation,
                        path: path.to_string(),
                        message: format!(
                            "String length {} is greater than maximum {}",
                            s.len(),
                            max_len
                        ),
                        severity: ViolationSeverity::Error,
                        expected: Some(serde_json::json!({"max_length": max_len})),
                        actual: Some(value.clone()),
                    });
                }
            }
        }

        None
    }

    /// Check a cardinality constraint
    fn check_cardinality(
        &self,
        path: &str,
        value: &serde_json::Value,
        constraint: &CardinalityConstraint,
    ) -> Option<OntologyViolation> {
        let arr = value.as_array()?;

        if arr.len() < constraint.min {
            return Some(OntologyViolation {
                rule_id: format!("cardinality:{}", path),
                violation_type: ViolationType::CardinalityViolation,
                path: path.to_string(),
                message: format!(
                    "Array has {} items, minimum is {}",
                    arr.len(),
                    constraint.min
                ),
                severity: ViolationSeverity::Error,
                expected: Some(serde_json::json!({"min": constraint.min})),
                actual: Some(serde_json::json!(arr.len())),
            });
        }

        if let Some(max) = constraint.max {
            if arr.len() > max {
                return Some(OntologyViolation {
                    rule_id: format!("cardinality:{}", path),
                    violation_type: ViolationType::CardinalityViolation,
                    path: path.to_string(),
                    message: format!("Array has {} items, maximum is {}", arr.len(), max),
                    severity: ViolationSeverity::Error,
                    expected: Some(serde_json::json!({"max": max})),
                    actual: Some(serde_json::json!(arr.len())),
                });
            }
        }

        if constraint.unique {
            let unique_count: HashSet<_> = arr.iter().map(|v| v.to_string()).collect();
            if unique_count.len() != arr.len() {
                return Some(OntologyViolation {
                    rule_id: format!("cardinality:{}", path),
                    violation_type: ViolationType::CardinalityViolation,
                    path: path.to_string(),
                    message: "Array contains duplicate values".to_string(),
                    severity: ViolationSeverity::Error,
                    expected: Some(serde_json::json!({"unique": true})),
                    actual: Some(value.clone()),
                });
            }
        }

        None
    }
}

/// Get a value from JSON by dot-separated path
fn get_json_path<'a>(value: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = value;

    for part in parts {
        match current {
            serde_json::Value::Object(obj) => {
                current = obj.get(part)?;
            }
            serde_json::Value::Array(arr) => {
                let index: usize = part.parse().ok()?;
                current = arr.get(index)?;
            }
            _ => return None,
        }
    }

    Some(current)
}

/// Check if a JSON value is truthy
fn is_truthy(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => false,
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        serde_json::Value::String(s) => !s.is_empty(),
        serde_json::Value::Array(a) => !a.is_empty(),
        serde_json::Value::Object(o) => !o.is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_allowed_values() {
        let mut ontology = Ontology::default();
        ontology.allowed_values.insert(
            "status".to_string(),
            vec![json!("active"), json!("inactive"), json!("pending")],
        );

        let checker = OntologyChecker::new(ontology);

        // Valid
        let result = checker.check(&json!({"status": "active"}));
        assert!(result.adheres);

        // Invalid
        let result = checker.check(&json!({"status": "unknown"}));
        assert!(!result.adheres);
        assert_eq!(result.error_count, 1);
    }

    #[test]
    fn test_relationship_implies() {
        let mut ontology = Ontology::default();
        ontology.relationships.push(Relationship {
            id: "premium_features".to_string(),
            from: "is_premium".to_string(),
            to: "premium_expires_at".to_string(),
            relation_type: RelationType::Implies,
            condition: None,
            message: "Premium users must have an expiration date".to_string(),
        });

        let checker = OntologyChecker::new(ontology);

        // Valid: premium with expiration
        let result = checker.check(&json!({
            "is_premium": true,
            "premium_expires_at": "2025-01-01"
        }));
        assert!(result.adheres);

        // Invalid: premium without expiration
        let result = checker.check(&json!({"is_premium": true}));
        assert!(!result.adheres);
    }

    #[test]
    fn test_relationship_excludes() {
        let mut ontology = Ontology::default();
        ontology.relationships.push(Relationship {
            id: "draft_published".to_string(),
            from: "is_draft".to_string(),
            to: "published_at".to_string(),
            relation_type: RelationType::Excludes,
            condition: None,
            message: "Draft cannot have published_at".to_string(),
        });

        let checker = OntologyChecker::new(ontology);

        // Valid: draft without published_at
        let result = checker.check(&json!({"is_draft": true}));
        assert!(result.adheres);

        // Invalid: both present
        let result = checker.check(&json!({
            "is_draft": true,
            "published_at": "2025-01-01"
        }));
        assert!(!result.adheres);
    }

    #[test]
    fn test_type_constraint_email() {
        let mut ontology = Ontology::default();
        ontology.type_constraints.insert(
            "email".to_string(),
            TypeConstraint {
                semantic_type: "email".to_string(),
                pattern: None,
                min: None,
                max: None,
                min_length: None,
                max_length: None,
            },
        );

        let checker = OntologyChecker::new(ontology);

        // Valid
        let result = checker.check(&json!({"email": "test@example.com"}));
        assert!(result.adheres);

        // Invalid
        let result = checker.check(&json!({"email": "not-an-email"}));
        assert!(!result.adheres);
    }

    #[test]
    fn test_cardinality_constraint() {
        let mut ontology = Ontology::default();
        ontology.cardinality.insert(
            "tags".to_string(),
            CardinalityConstraint {
                min: 1,
                max: Some(5),
                unique: true,
            },
        );

        let checker = OntologyChecker::new(ontology);

        // Valid
        let result = checker.check(&json!({"tags": ["a", "b", "c"]}));
        assert!(result.adheres);

        // Invalid: empty
        let result = checker.check(&json!({"tags": []}));
        assert!(!result.adheres);

        // Invalid: duplicates
        let result = checker.check(&json!({"tags": ["a", "a", "b"]}));
        assert!(!result.adheres);
    }
}
