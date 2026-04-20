//! CEL-based Assertion System
//!
//! Provides assertion definitions and evaluation using the Common Expression Language (CEL).
//! Assertions are used to compute Instruction Compliance (IC) metrics.
//!
//! # Example
//!
//! ```ignore
//! use mpl_core::assertions::{Assertion, AssertionSet, AssertionEvaluator};
//! use serde_json::json;
//!
//! let assertions = AssertionSet::new(vec![
//!     Assertion::new("amount_positive", "payload.amount > 0", "Amount must be positive"),
//!     Assertion::new("currency_valid", "payload.currency in ['USD', 'EUR', 'GBP']", "Invalid currency"),
//! ]);
//!
//! let payload = json!({"amount": 100, "currency": "USD"});
//! let result = assertions.evaluate(&payload)?;
//! assert!(result.passed());
//! ```

use cel_interpreter::{Context, Program, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use tracing::debug;

/// Assertion evaluation error
#[derive(Debug, Error)]
pub enum AssertionError {
    #[error("Failed to compile CEL expression '{expr}': {message}")]
    CompilationError { expr: String, message: String },

    #[error("Failed to evaluate CEL expression '{expr}': {message}")]
    EvaluationError { expr: String, message: String },

    #[error("Invalid payload: {0}")]
    InvalidPayload(String),
}

/// A single assertion with a CEL expression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assertion {
    /// Unique identifier for this assertion
    pub id: String,

    /// CEL expression to evaluate
    /// Available variables: payload, metadata, context
    pub expression: String,

    /// Human-readable message when assertion fails
    pub message: String,

    /// Severity: error (blocks), warning (logs), info (metrics only)
    #[serde(default = "default_severity")]
    pub severity: AssertionSeverity,

    /// Optional tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_severity() -> AssertionSeverity {
    AssertionSeverity::Error
}

/// Assertion severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AssertionSeverity {
    /// Blocks the request if assertion fails
    #[default]
    Error,
    /// Logs a warning but allows the request
    Warning,
    /// Only affects metrics, no blocking or logging
    Info,
}

impl Assertion {
    /// Create a new assertion
    pub fn new(id: impl Into<String>, expression: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            expression: expression.into(),
            message: message.into(),
            severity: AssertionSeverity::Error,
            tags: Vec::new(),
        }
    }

    /// Set severity
    pub fn with_severity(mut self, severity: AssertionSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Add tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

/// Result of evaluating a single assertion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionResult {
    /// Assertion ID
    pub id: String,

    /// Whether the assertion passed
    pub passed: bool,

    /// The assertion message (shown on failure)
    pub message: String,

    /// Severity of this assertion
    pub severity: AssertionSeverity,

    /// Actual value returned by the expression (for debugging)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_value: Option<String>,

    /// Error message if evaluation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// A set of assertions to evaluate together
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AssertionSet {
    /// Name of this assertion set
    #[serde(default)]
    pub name: String,

    /// Description
    #[serde(default)]
    pub description: String,

    /// The assertions in this set
    pub assertions: Vec<Assertion>,

    /// Whether to stop on first error-severity failure
    #[serde(default)]
    pub fail_fast: bool,
}

impl AssertionSet {
    /// Create a new assertion set
    pub fn new(assertions: Vec<Assertion>) -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            assertions,
            fail_fast: false,
        }
    }

    /// Create with name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Add an assertion
    pub fn add(&mut self, assertion: Assertion) {
        self.assertions.push(assertion);
    }

    /// Evaluate all assertions against a payload
    pub fn evaluate(&self, payload: &serde_json::Value) -> Result<AssertionSetResult, AssertionError> {
        let evaluator = AssertionEvaluator::new();
        evaluator.evaluate_set(self, payload, None)
    }

    /// Evaluate with additional context
    pub fn evaluate_with_context(
        &self,
        payload: &serde_json::Value,
        context: &EvaluationContext,
    ) -> Result<AssertionSetResult, AssertionError> {
        let evaluator = AssertionEvaluator::new();
        evaluator.evaluate_set(self, payload, Some(context))
    }
}

/// Additional context for assertion evaluation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvaluationContext {
    /// Metadata from the request
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,

    /// SType being evaluated
    #[serde(default)]
    pub stype: Option<String>,

    /// Tool name (if tool call)
    #[serde(default)]
    pub tool_name: Option<String>,

    /// Request arguments (for IC)
    #[serde(default)]
    pub arguments: Option<serde_json::Value>,

    /// Response data (for TOC)
    #[serde(default)]
    pub response: Option<serde_json::Value>,
}

/// Result of evaluating an assertion set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionSetResult {
    /// Individual assertion results
    pub results: Vec<AssertionResult>,

    /// Number of assertions that passed
    pub passed_count: usize,

    /// Number of assertions that failed
    pub failed_count: usize,

    /// Number of error-severity failures
    pub error_count: usize,

    /// Number of warning-severity failures
    pub warning_count: usize,

    /// Computed IC score (0.0 - 1.0)
    pub ic_score: f64,
}

impl AssertionSetResult {
    /// Check if all assertions passed
    pub fn passed(&self) -> bool {
        self.error_count == 0
    }

    /// Check if any error-severity assertions failed
    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }

    /// Get failed assertion messages
    pub fn failure_messages(&self) -> Vec<&str> {
        self.results
            .iter()
            .filter(|r| !r.passed && r.severity == AssertionSeverity::Error)
            .map(|r| r.message.as_str())
            .collect()
    }
}

/// CEL-based assertion evaluator
pub struct AssertionEvaluator {
    // Note: Program doesn't implement Clone, so we compile on each evaluation
    // This is fast enough for typical use cases
    _marker: std::marker::PhantomData<()>,
}

impl Default for AssertionEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl AssertionEvaluator {
    /// Create a new evaluator
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }

    /// Evaluate an assertion set
    pub fn evaluate_set(
        &self,
        set: &AssertionSet,
        payload: &serde_json::Value,
        context: Option<&EvaluationContext>,
    ) -> Result<AssertionSetResult, AssertionError> {
        let mut results = Vec::with_capacity(set.assertions.len());
        let mut passed_count = 0;
        let mut failed_count = 0;
        let mut error_count = 0;
        let mut warning_count = 0;

        for assertion in &set.assertions {
            let result = self.evaluate_single(assertion, payload, context);

            match &result {
                Ok(r) => {
                    if r.passed {
                        passed_count += 1;
                    } else {
                        failed_count += 1;
                        match r.severity {
                            AssertionSeverity::Error => error_count += 1,
                            AssertionSeverity::Warning => warning_count += 1,
                            AssertionSeverity::Info => {}
                        }
                    }
                    results.push(r.clone());

                    // Fail fast on error-severity failures
                    if set.fail_fast && !r.passed && r.severity == AssertionSeverity::Error {
                        break;
                    }
                }
                Err(e) => {
                    // Evaluation error counts as failure
                    failed_count += 1;
                    error_count += 1;
                    results.push(AssertionResult {
                        id: assertion.id.clone(),
                        passed: false,
                        message: assertion.message.clone(),
                        severity: assertion.severity,
                        actual_value: None,
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        // Compute IC score
        let total = set.assertions.len();
        let ic_score = if total == 0 {
            1.0
        } else {
            passed_count as f64 / total as f64
        };

        Ok(AssertionSetResult {
            results,
            passed_count,
            failed_count,
            error_count,
            warning_count,
            ic_score,
        })
    }

    /// Evaluate a single assertion
    pub fn evaluate_single(
        &self,
        assertion: &Assertion,
        payload: &serde_json::Value,
        context: Option<&EvaluationContext>,
    ) -> Result<AssertionResult, AssertionError> {
        // Compile the CEL expression
        let program = Program::compile(&assertion.expression).map_err(|e| {
            AssertionError::CompilationError {
                expr: assertion.expression.clone(),
                message: format!("{:?}", e),
            }
        })?;

        // Build CEL context
        let mut cel_context = Context::default();

        // Add payload as a variable
        let payload_value = json_to_cel(payload);
        cel_context.add_variable("payload", payload_value).ok();

        // Add context variables if provided
        if let Some(ctx) = context {
            if let Some(args) = &ctx.arguments {
                cel_context.add_variable("args", json_to_cel(args)).ok();
            }
            if let Some(resp) = &ctx.response {
                cel_context.add_variable("response", json_to_cel(resp)).ok();
            }
            if let Some(stype) = &ctx.stype {
                cel_context.add_variable("stype", stype.clone()).ok();
            }
            if let Some(tool) = &ctx.tool_name {
                cel_context.add_variable("tool", tool.clone()).ok();
            }

            // Add metadata
            let meta_value = json_to_cel(&serde_json::to_value(&ctx.metadata).unwrap_or_default());
            cel_context.add_variable("metadata", meta_value).ok();
        }

        // Execute the program
        let result = program.execute(&cel_context).map_err(|e| {
            AssertionError::EvaluationError {
                expr: assertion.expression.clone(),
                message: format!("{:?}", e),
            }
        })?;

        // Check if result is truthy
        let passed = match &result {
            Value::Bool(b) => *b,
            Value::Null => false,
            Value::Int(i) => *i != 0,
            Value::UInt(u) => *u != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::List(l) => !l.is_empty(),
            Value::Map(m) => !m.map.is_empty(),
            _ => true, // Other types are considered truthy
        };

        debug!(
            assertion_id = %assertion.id,
            passed = passed,
            "Assertion evaluated"
        );

        Ok(AssertionResult {
            id: assertion.id.clone(),
            passed,
            message: assertion.message.clone(),
            severity: assertion.severity,
            actual_value: Some(format!("{:?}", result)),
            error: None,
        })
    }
}

/// Convert JSON value to CEL value
fn json_to_cel(value: &serde_json::Value) -> Value {
    match value {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(u) = n.as_u64() {
                Value::UInt(u)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Null
            }
        }
        serde_json::Value::String(s) => Value::String(s.clone().into()),
        serde_json::Value::Array(arr) => {
            Value::List(arr.iter().map(json_to_cel).collect::<Vec<_>>().into())
        }
        serde_json::Value::Object(obj) => {
            let map: HashMap<String, Value> = obj
                .iter()
                .map(|(k, v)| (k.clone(), json_to_cel(v)))
                .collect();
            // CEL Map requires Key type, use String keys
            let cel_map: HashMap<cel_interpreter::objects::Key, Value> = map
                .into_iter()
                .map(|(k, v)| (cel_interpreter::objects::Key::String(k.into()), v))
                .collect();
            Value::Map(cel_interpreter::objects::Map { map: cel_map.into() })
        }
    }
}

/// Load assertions from a JSON file
pub fn load_assertions_from_json(json: &str) -> Result<AssertionSet, serde_json::Error> {
    serde_json::from_str(json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_simple_assertion() {
        let assertion = Assertion::new(
            "amount_positive",
            "payload.amount > 0",
            "Amount must be positive",
        );

        let evaluator = AssertionEvaluator::new();

        // Passing case
        let payload = json!({"amount": 100});
        let result = evaluator.evaluate_single(&assertion, &payload, None).unwrap();
        assert!(result.passed);

        // Failing case
        let payload = json!({"amount": -50});
        let result = evaluator.evaluate_single(&assertion, &payload, None).unwrap();
        assert!(!result.passed);
    }

    #[test]
    fn test_string_assertion() {
        let assertion = Assertion::new(
            "currency_valid",
            "payload.currency in ['USD', 'EUR', 'GBP']",
            "Invalid currency",
        );

        let evaluator = AssertionEvaluator::new();

        let payload = json!({"currency": "USD"});
        let result = evaluator.evaluate_single(&assertion, &payload, None).unwrap();
        assert!(result.passed);

        let payload = json!({"currency": "XYZ"});
        let result = evaluator.evaluate_single(&assertion, &payload, None).unwrap();
        assert!(!result.passed);
    }

    #[test]
    fn test_assertion_set() {
        let set = AssertionSet::new(vec![
            Assertion::new("a1", "payload.x > 0", "X must be positive"),
            Assertion::new("a2", "payload.y < 100", "Y must be less than 100"),
        ]);

        let payload = json!({"x": 10, "y": 50});
        let result = set.evaluate(&payload).unwrap();
        assert!(result.passed());
        assert_eq!(result.ic_score, 1.0);

        let payload = json!({"x": -5, "y": 50});
        let result = set.evaluate(&payload).unwrap();
        assert!(!result.passed());
        assert_eq!(result.ic_score, 0.5);
    }

    #[test]
    fn test_nested_payload() {
        let assertion = Assertion::new(
            "nested_check",
            "payload.user.age >= 18",
            "User must be 18+",
        );

        let evaluator = AssertionEvaluator::new();

        let payload = json!({"user": {"name": "Alice", "age": 25}});
        let result = evaluator.evaluate_single(&assertion, &payload, None).unwrap();
        assert!(result.passed);
    }

    #[test]
    fn test_array_operations() {
        let assertion = Assertion::new(
            "has_items",
            "size(payload.items) > 0",
            "Items cannot be empty",
        );

        let evaluator = AssertionEvaluator::new();

        let payload = json!({"items": [1, 2, 3]});
        let result = evaluator.evaluate_single(&assertion, &payload, None).unwrap();
        assert!(result.passed);

        let payload = json!({"items": []});
        let result = evaluator.evaluate_single(&assertion, &payload, None).unwrap();
        assert!(!result.passed);
    }

    #[test]
    fn test_context_variables() {
        let assertion = Assertion::new(
            "tool_check",
            "tool == 'calendar.create'",
            "Only calendar.create allowed",
        );

        let evaluator = AssertionEvaluator::new();
        let payload = json!({});

        let ctx = EvaluationContext {
            tool_name: Some("calendar.create".to_string()),
            ..Default::default()
        };

        let result = evaluator.evaluate_single(&assertion, &payload, Some(&ctx)).unwrap();
        assert!(result.passed);
    }

    #[test]
    fn test_severity_levels() {
        let set = AssertionSet::new(vec![
            Assertion::new("error_check", "false", "Error level").with_severity(AssertionSeverity::Error),
            Assertion::new("warn_check", "false", "Warning level").with_severity(AssertionSeverity::Warning),
            Assertion::new("info_check", "false", "Info level").with_severity(AssertionSeverity::Info),
        ]);

        let result = set.evaluate(&json!({})).unwrap();
        assert_eq!(result.error_count, 1);
        assert_eq!(result.warning_count, 1);
        assert_eq!(result.failed_count, 3);
        assert!(!result.passed()); // Has errors
    }

    #[test]
    fn test_load_from_json() {
        let json = r#"{
            "name": "finance_checks",
            "description": "Financial payload validations",
            "assertions": [
                {
                    "id": "amount_check",
                    "expression": "payload.amount > 0",
                    "message": "Amount must be positive"
                },
                {
                    "id": "currency_check",
                    "expression": "payload.currency in ['USD', 'EUR']",
                    "message": "Invalid currency",
                    "severity": "warning"
                }
            ]
        }"#;

        let set = load_assertions_from_json(json).unwrap();
        assert_eq!(set.name, "finance_checks");
        assert_eq!(set.assertions.len(), 2);
        assert_eq!(set.assertions[1].severity, AssertionSeverity::Warning);
    }
}
