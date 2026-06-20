//! QoM Metric Computation Infrastructure
//!
//! Provides a unified framework for computing all QoM metrics:
//! - Schema Fidelity (SF): JSON Schema validation
//! - Instruction Compliance (IC): CEL assertion evaluation
//! - Tool Outcome Correctness (TOC): Side-effect verification
//! - Groundedness (G): Citation/source verification
//! - Determinism Jitter (DJ): Output stability measurement
//! - Ontology Adherence (OA): Domain constraint conformance

use crate::assertions::{AssertionSet, AssertionSetResult, EvaluationContext};
use crate::qom::QomMetrics;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Error during metric computation
#[derive(Debug, Error)]
pub enum MetricError {
    #[error("Schema validation error: {0}")]
    SchemaError(String),

    #[error("Assertion evaluation error: {0}")]
    AssertionError(String),

    #[error("TOC verification error: {0}")]
    TocError(String),

    #[error("Groundedness computation error: {0}")]
    GroundednessError(String),

    #[error("Metric not supported: {0}")]
    NotSupported(String),
}

/// Context for metric computation
///
/// Contains all the data needed to compute QoM metrics for a request/response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetricContext {
    /// The SType being validated
    pub stype: String,

    /// The request payload
    pub payload: serde_json::Value,

    /// The response payload (for TOC)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<serde_json::Value>,

    /// Tool name (if this is a tool call)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,

    /// Request arguments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,

    /// Assertions to evaluate for IC
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assertions: Option<AssertionSet>,

    /// TOC verification result from external source (header or callback)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toc_result: Option<TocResult>,

    /// Sources/citations for groundedness checking
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<Source>,

    /// Previous response for determinism comparison
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_response: Option<serde_json::Value>,

    /// Ontology constraints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ontology_constraints: Option<OntologyConstraints>,

    /// Additional metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl MetricContext {
    /// Create a new context for a payload
    pub fn new(stype: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            stype: stype.into(),
            payload,
            ..Default::default()
        }
    }

    /// Add response data
    pub fn with_response(mut self, response: serde_json::Value) -> Self {
        self.response = Some(response);
        self
    }

    /// Add assertions for IC computation
    pub fn with_assertions(mut self, assertions: AssertionSet) -> Self {
        self.assertions = Some(assertions);
        self
    }

    /// Add TOC result from external verification
    pub fn with_toc_result(mut self, result: TocResult) -> Self {
        self.toc_result = Some(result);
        self
    }

    /// Add sources for groundedness checking
    pub fn with_sources(mut self, sources: Vec<Source>) -> Self {
        self.sources = sources;
        self
    }

    /// Add previous response for determinism checking
    pub fn with_previous_response(mut self, previous: serde_json::Value) -> Self {
        self.previous_response = Some(previous);
        self
    }

    /// Add ontology constraints
    pub fn with_ontology(mut self, constraints: OntologyConstraints) -> Self {
        self.ontology_constraints = Some(constraints);
        self
    }

    /// Convert to assertion evaluation context
    pub fn to_evaluation_context(&self) -> EvaluationContext {
        EvaluationContext {
            stype: Some(self.stype.clone()),
            tool_name: self.tool_name.clone(),
            arguments: self.arguments.clone(),
            response: self.response.clone(),
            metadata: self.metadata.clone(),
        }
    }
}

/// TOC verification result from external source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocResult {
    /// Whether the tool outcome was verified
    pub verified: bool,

    /// Verification method used
    pub method: TocMethod,

    /// Detailed results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,

    /// Expected outcome description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,

    /// Actual outcome description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
}

impl TocResult {
    /// Create a verified TOC result
    pub fn verified(method: TocMethod) -> Self {
        Self {
            verified: true,
            method,
            details: None,
            expected: None,
            actual: None,
        }
    }

    /// Create a failed TOC result
    pub fn failed(method: TocMethod, details: impl Into<String>) -> Self {
        Self {
            verified: false,
            method,
            details: Some(details.into()),
            expected: None,
            actual: None,
        }
    }

    /// Convert to score (1.0 for verified, 0.0 otherwise)
    pub fn to_score(&self) -> f64 {
        if self.verified {
            1.0
        } else {
            0.0
        }
    }
}

/// TOC verification method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TocMethod {
    /// Verified via X-MPL-TOC-Result header
    Header,
    /// Verified via callback endpoint
    Callback,
    /// Verified via polling/query
    Poll,
    /// Not verified (placeholder)
    None,
}

/// Source for groundedness verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    /// Source identifier (URL, document ID, etc.)
    pub id: String,

    /// Source content or excerpt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// Confidence score for this source
    #[serde(default = "default_confidence")]
    pub confidence: f64,
}

fn default_confidence() -> f64 {
    1.0
}

/// Ontology constraints for OA verification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OntologyConstraints {
    /// Allowed values for specific fields
    #[serde(default)]
    pub allowed_values: HashMap<String, Vec<serde_json::Value>>,

    /// Required relationships between fields
    #[serde(default)]
    pub relationships: Vec<OntologyRelation>,

    /// Domain-specific type constraints
    #[serde(default)]
    pub type_constraints: HashMap<String, String>,
}

/// Ontology relationship constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyRelation {
    /// Source field path
    pub from: String,
    /// Target field path
    pub to: String,
    /// Relationship type
    pub relation: RelationType,
}

/// Types of ontology relationships
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    /// from implies to must exist
    Implies,
    /// from and to are mutually exclusive
    Excludes,
    /// from must be less than to
    LessThan,
    /// from must equal to
    Equals,
}

/// Result of computing all metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricComputeResult {
    /// Computed metrics
    pub metrics: QomMetrics,

    /// Assertion evaluation results (if IC was computed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assertion_results: Option<AssertionSetResult>,

    /// TOC verification details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toc_details: Option<TocResult>,

    /// Groundedness analysis
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groundedness_details: Option<GroundednessResult>,

    /// Determinism analysis
    #[serde(skip_serializing_if = "Option::is_none")]
    pub determinism_details: Option<DeterminismResult>,

    /// Ontology analysis
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ontology_details: Option<OntologyResult>,

    /// Any errors during computation
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

/// Groundedness verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundednessResult {
    /// Overall groundedness score
    pub score: f64,

    /// Claims identified in the response
    pub claims: Vec<Claim>,

    /// Method used for verification
    pub method: GroundednessMethod,
}

/// A claim that needs grounding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    /// The claim text
    pub text: String,

    /// Whether the claim is grounded
    pub grounded: bool,

    /// Supporting source (if grounded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// Confidence in the grounding assessment
    pub confidence: f64,
}

/// Method used for groundedness verification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroundednessMethod {
    /// Local citation matching
    Local,
    /// LLM-based verification
    Llm,
    /// Hybrid (local first, LLM for uncertain)
    Hybrid,
    /// Not computed
    None,
}

/// Determinism jitter result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeterminismResult {
    /// Similarity score between responses (0.0 - 1.0)
    pub similarity: f64,

    /// Fields that differed
    #[serde(default)]
    pub differences: Vec<FieldDiff>,

    /// Whether jitter is within acceptable bounds
    pub acceptable: bool,
}

/// A field difference between responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDiff {
    /// JSON path to the field
    pub path: String,
    /// Value in first response
    pub value1: serde_json::Value,
    /// Value in second response
    pub value2: serde_json::Value,
}

/// Ontology adherence result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyResult {
    /// Overall adherence score
    pub score: f64,

    /// Violations found
    #[serde(default)]
    pub violations: Vec<OntologyViolation>,
}

/// An ontology constraint violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyViolation {
    /// Type of violation
    pub kind: String,
    /// Description of the violation
    pub message: String,
    /// Field path involved
    pub path: String,
}

/// Main metric computer that orchestrates all computations
pub struct QomComputer {
    /// Whether to compute IC metrics
    compute_ic: bool,
    /// Whether to compute TOC metrics
    compute_toc: bool,
    /// Whether to compute groundedness
    compute_groundedness: bool,
    /// Whether to compute determinism
    compute_determinism: bool,
    /// Whether to compute ontology adherence
    compute_ontology: bool,
}

impl Default for QomComputer {
    fn default() -> Self {
        Self::new()
    }
}

impl QomComputer {
    /// Create a new QoM computer with default settings
    pub fn new() -> Self {
        Self {
            compute_ic: true,
            compute_toc: true,
            compute_groundedness: false, // Disabled by default (expensive)
            compute_determinism: false,  // Disabled by default (requires replay)
            compute_ontology: false,     // Disabled by default
        }
    }

    /// Enable/disable IC computation
    pub fn with_ic(mut self, enabled: bool) -> Self {
        self.compute_ic = enabled;
        self
    }

    /// Enable/disable TOC computation
    pub fn with_toc(mut self, enabled: bool) -> Self {
        self.compute_toc = enabled;
        self
    }

    /// Enable/disable groundedness computation
    pub fn with_groundedness(mut self, enabled: bool) -> Self {
        self.compute_groundedness = enabled;
        self
    }

    /// Enable/disable determinism computation
    pub fn with_determinism(mut self, enabled: bool) -> Self {
        self.compute_determinism = enabled;
        self
    }

    /// Enable/disable ontology adherence computation
    pub fn with_ontology(mut self, enabled: bool) -> Self {
        self.compute_ontology = enabled;
        self
    }

    /// Compute all enabled metrics
    pub fn compute(&self, ctx: &MetricContext) -> MetricComputeResult {
        let mut metrics = QomMetrics::default();
        let mut errors = Vec::new();
        let mut assertion_results = None;
        let mut toc_details = None;
        let mut groundedness_details = None;
        let mut determinism_details = None;
        let mut ontology_details = None;

        // Schema Fidelity is always 1.0 at this point (validation happens earlier in the pipeline)
        // The actual SF check is done by SchemaValidator before this is called
        metrics.schema_fidelity = 1.0;

        // Instruction Compliance (IC)
        if self.compute_ic {
            if let Some(assertions) = &ctx.assertions {
                match self.compute_ic_metric(ctx, assertions) {
                    Ok((score, results)) => {
                        metrics.instruction_compliance = Some(score);
                        assertion_results = Some(results);
                    }
                    Err(e) => {
                        errors.push(format!("IC computation failed: {}", e));
                    }
                }
            }
        }

        // Tool Outcome Correctness (TOC)
        if self.compute_toc {
            if let Some(toc) = &ctx.toc_result {
                metrics.tool_outcome_correctness = Some(toc.to_score());
                toc_details = Some(toc.clone());
            }
        }

        // Groundedness (G)
        if self.compute_groundedness && !ctx.sources.is_empty() {
            match self.compute_groundedness_metric(ctx) {
                Ok(result) => {
                    metrics.groundedness = Some(result.score);
                    groundedness_details = Some(result);
                }
                Err(e) => {
                    errors.push(format!("Groundedness computation failed: {}", e));
                }
            }
        }

        // Determinism Jitter (DJ)
        if self.compute_determinism {
            if let Some(previous) = &ctx.previous_response {
                if let Some(current) = &ctx.response {
                    match self.compute_determinism_metric(current, previous) {
                        Ok(result) => {
                            metrics.determinism_jitter = Some(result.similarity);
                            determinism_details = Some(result);
                        }
                        Err(e) => {
                            errors.push(format!("Determinism computation failed: {}", e));
                        }
                    }
                }
            }
        }

        // Ontology Adherence (OA)
        if self.compute_ontology {
            if let Some(constraints) = &ctx.ontology_constraints {
                match self.compute_ontology_metric(ctx, constraints) {
                    Ok(result) => {
                        metrics.ontology_adherence = Some(result.score);
                        ontology_details = Some(result);
                    }
                    Err(e) => {
                        errors.push(format!("Ontology computation failed: {}", e));
                    }
                }
            }
        }

        MetricComputeResult {
            metrics,
            assertion_results,
            toc_details,
            groundedness_details,
            determinism_details,
            ontology_details,
            errors,
        }
    }

    /// Compute IC metric from assertions
    fn compute_ic_metric(
        &self,
        ctx: &MetricContext,
        assertions: &AssertionSet,
    ) -> Result<(f64, AssertionSetResult), MetricError> {
        let eval_ctx = ctx.to_evaluation_context();
        let result = assertions
            .evaluate_with_context(&ctx.payload, &eval_ctx)
            .map_err(|e| MetricError::AssertionError(e.to_string()))?;

        Ok((result.ic_score, result))
    }

    /// Compute groundedness metric (hybrid approach)
    fn compute_groundedness_metric(
        &self,
        ctx: &MetricContext,
    ) -> Result<GroundednessResult, MetricError> {
        // For now, implement simple local citation matching
        // Full hybrid implementation would involve LLM calls for uncertain cases

        let response_text = ctx
            .response
            .as_ref()
            .map(|r| r.to_string())
            .unwrap_or_default();

        let mut claims = Vec::new();
        let mut grounded_count = 0;

        // Simple heuristic: check if source content appears in response
        for source in &ctx.sources {
            if let Some(content) = &source.content {
                let claim = Claim {
                    text: content.chars().take(100).collect(),
                    grounded: response_text.contains(content) || content.contains(&response_text),
                    source: Some(source.id.clone()),
                    confidence: source.confidence,
                };
                if claim.grounded {
                    grounded_count += 1;
                }
                claims.push(claim);
            }
        }

        let score = if claims.is_empty() {
            1.0 // No claims to verify
        } else {
            grounded_count as f64 / claims.len() as f64
        };

        Ok(GroundednessResult {
            score,
            claims,
            method: GroundednessMethod::Local,
        })
    }

    /// Compute determinism jitter metric
    fn compute_determinism_metric(
        &self,
        current: &serde_json::Value,
        previous: &serde_json::Value,
    ) -> Result<DeterminismResult, MetricError> {
        let mut differences = Vec::new();

        // Simple deep comparison
        fn compare_values(
            v1: &serde_json::Value,
            v2: &serde_json::Value,
            path: &str,
            diffs: &mut Vec<FieldDiff>,
        ) -> bool {
            match (v1, v2) {
                (serde_json::Value::Object(o1), serde_json::Value::Object(o2)) => {
                    let mut all_match = true;
                    for (k, val1) in o1 {
                        let new_path = if path.is_empty() {
                            k.clone()
                        } else {
                            format!("{}.{}", path, k)
                        };
                        if let Some(val2) = o2.get(k) {
                            if !compare_values(val1, val2, &new_path, diffs) {
                                all_match = false;
                            }
                        } else {
                            diffs.push(FieldDiff {
                                path: new_path,
                                value1: val1.clone(),
                                value2: serde_json::Value::Null,
                            });
                            all_match = false;
                        }
                    }
                    // Check for keys in o2 not in o1
                    for k in o2.keys() {
                        if !o1.contains_key(k) {
                            let new_path = if path.is_empty() {
                                k.clone()
                            } else {
                                format!("{}.{}", path, k)
                            };
                            diffs.push(FieldDiff {
                                path: new_path,
                                value1: serde_json::Value::Null,
                                value2: o2.get(k).cloned().unwrap_or(serde_json::Value::Null),
                            });
                            all_match = false;
                        }
                    }
                    all_match
                }
                (serde_json::Value::Array(a1), serde_json::Value::Array(a2)) => {
                    if a1.len() != a2.len() {
                        diffs.push(FieldDiff {
                            path: path.to_string(),
                            value1: v1.clone(),
                            value2: v2.clone(),
                        });
                        return false;
                    }
                    let mut all_match = true;
                    for (i, (item1, item2)) in a1.iter().zip(a2.iter()).enumerate() {
                        let new_path = format!("{}[{}]", path, i);
                        if !compare_values(item1, item2, &new_path, diffs) {
                            all_match = false;
                        }
                    }
                    all_match
                }
                _ => {
                    if v1 != v2 {
                        diffs.push(FieldDiff {
                            path: path.to_string(),
                            value1: v1.clone(),
                            value2: v2.clone(),
                        });
                        false
                    } else {
                        true
                    }
                }
            }
        }

        let matches = compare_values(current, previous, "", &mut differences);

        // Calculate similarity score
        let similarity = if matches {
            1.0
        } else {
            // Simple heuristic: more differences = lower score
            let total_fields = count_fields(current) + count_fields(previous);
            if total_fields == 0 {
                1.0
            } else {
                1.0 - (differences.len() as f64 * 2.0 / total_fields as f64).min(1.0)
            }
        };

        Ok(DeterminismResult {
            similarity,
            differences,
            acceptable: similarity >= 0.9, // 90% threshold by default
        })
    }

    /// Compute ontology adherence metric
    fn compute_ontology_metric(
        &self,
        ctx: &MetricContext,
        constraints: &OntologyConstraints,
    ) -> Result<OntologyResult, MetricError> {
        let mut violations = Vec::new();

        // Check allowed values
        for (path, allowed) in &constraints.allowed_values {
            if let Some(value) = get_json_path(&ctx.payload, path) {
                if !allowed.contains(value) {
                    violations.push(OntologyViolation {
                        kind: "allowed_values".to_string(),
                        message: format!(
                            "Value at '{}' is not in allowed set: {:?}",
                            path, allowed
                        ),
                        path: path.clone(),
                    });
                }
            }
        }

        // Check relationships
        for relation in &constraints.relationships {
            let from_value = get_json_path(&ctx.payload, &relation.from);
            let to_value = get_json_path(&ctx.payload, &relation.to);

            match relation.relation {
                RelationType::Implies => {
                    if from_value.is_some() && to_value.is_none() {
                        violations.push(OntologyViolation {
                            kind: "implies".to_string(),
                            message: format!(
                                "'{}' implies '{}' must exist",
                                relation.from, relation.to
                            ),
                            path: relation.to.clone(),
                        });
                    }
                }
                RelationType::Excludes => {
                    if from_value.is_some() && to_value.is_some() {
                        violations.push(OntologyViolation {
                            kind: "excludes".to_string(),
                            message: format!(
                                "'{}' and '{}' are mutually exclusive",
                                relation.from, relation.to
                            ),
                            path: relation.from.clone(),
                        });
                    }
                }
                RelationType::LessThan => {
                    if let (Some(v1), Some(v2)) = (from_value, to_value) {
                        if let (Some(n1), Some(n2)) = (v1.as_f64(), v2.as_f64()) {
                            if n1 >= n2 {
                                violations.push(OntologyViolation {
                                    kind: "less_than".to_string(),
                                    message: format!(
                                        "'{}' must be less than '{}'",
                                        relation.from, relation.to
                                    ),
                                    path: relation.from.clone(),
                                });
                            }
                        }
                    }
                }
                RelationType::Equals => {
                    if from_value != to_value {
                        violations.push(OntologyViolation {
                            kind: "equals".to_string(),
                            message: format!("'{}' must equal '{}'", relation.from, relation.to),
                            path: relation.from.clone(),
                        });
                    }
                }
            }
        }

        // Calculate score
        let total_constraints = constraints.allowed_values.len() + constraints.relationships.len();
        let score = if total_constraints == 0 {
            1.0
        } else {
            1.0 - (violations.len() as f64 / total_constraints as f64)
        };

        Ok(OntologyResult { score, violations })
    }
}

/// Count total fields in a JSON value
fn count_fields(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Object(obj) => obj.len() + obj.values().map(count_fields).sum::<usize>(),
        serde_json::Value::Array(arr) => arr.iter().map(count_fields).sum(),
        _ => 1,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assertions::Assertion;
    use serde_json::json;

    #[test]
    fn test_basic_compute() {
        let computer = QomComputer::new();
        let ctx = MetricContext::new("test.Type.v1", json!({"value": 42}));

        let result = computer.compute(&ctx);
        assert_eq!(result.metrics.schema_fidelity, 1.0);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_ic_computation() {
        let assertions = AssertionSet::new(vec![
            Assertion::new("check1", "payload.value > 0", "Value must be positive"),
            Assertion::new(
                "check2",
                "payload.value < 100",
                "Value must be less than 100",
            ),
        ]);

        let ctx =
            MetricContext::new("test.Type.v1", json!({"value": 42})).with_assertions(assertions);

        let computer = QomComputer::new().with_ic(true);
        let result = computer.compute(&ctx);

        assert_eq!(result.metrics.instruction_compliance, Some(1.0));
        assert!(result.assertion_results.is_some());
    }

    #[test]
    fn test_ic_partial_failure() {
        let assertions = AssertionSet::new(vec![
            Assertion::new("check1", "payload.value > 0", "Value must be positive"),
            Assertion::new(
                "check2",
                "payload.value > 100",
                "Value must be greater than 100",
            ),
        ]);

        let ctx =
            MetricContext::new("test.Type.v1", json!({"value": 42})).with_assertions(assertions);

        let computer = QomComputer::new().with_ic(true);
        let result = computer.compute(&ctx);

        assert_eq!(result.metrics.instruction_compliance, Some(0.5));
    }

    #[test]
    fn test_toc_verified() {
        let toc = TocResult::verified(TocMethod::Header);
        let ctx = MetricContext::new("test.Type.v1", json!({})).with_toc_result(toc);

        let computer = QomComputer::new().with_toc(true);
        let result = computer.compute(&ctx);

        assert_eq!(result.metrics.tool_outcome_correctness, Some(1.0));
    }

    #[test]
    fn test_toc_failed() {
        let toc = TocResult::failed(TocMethod::Callback, "Side effect not observed");
        let ctx = MetricContext::new("test.Type.v1", json!({})).with_toc_result(toc);

        let computer = QomComputer::new().with_toc(true);
        let result = computer.compute(&ctx);

        assert_eq!(result.metrics.tool_outcome_correctness, Some(0.0));
    }

    #[test]
    fn test_determinism_identical() {
        let response = json!({"result": "hello", "count": 5});
        let previous = json!({"result": "hello", "count": 5});

        let ctx = MetricContext::new("test.Type.v1", json!({}))
            .with_response(response)
            .with_previous_response(previous);

        let computer = QomComputer::new().with_determinism(true);
        let result = computer.compute(&ctx);

        assert_eq!(result.metrics.determinism_jitter, Some(1.0));
    }

    #[test]
    fn test_determinism_different() {
        let response = json!({"result": "hello", "count": 5});
        let previous = json!({"result": "world", "count": 10});

        let ctx = MetricContext::new("test.Type.v1", json!({}))
            .with_response(response)
            .with_previous_response(previous);

        let computer = QomComputer::new().with_determinism(true);
        let result = computer.compute(&ctx);

        // Should have some differences
        assert!(result.metrics.determinism_jitter.unwrap() < 1.0);
        assert!(result.determinism_details.is_some());
        assert!(!result.determinism_details.unwrap().differences.is_empty());
    }

    #[test]
    fn test_ontology_allowed_values() {
        let mut constraints = OntologyConstraints::default();
        constraints.allowed_values.insert(
            "status".to_string(),
            vec![json!("active"), json!("inactive")],
        );

        let ctx = MetricContext::new("test.Type.v1", json!({"status": "active"}))
            .with_ontology(constraints);

        let computer = QomComputer::new().with_ontology(true);
        let result = computer.compute(&ctx);

        assert_eq!(result.metrics.ontology_adherence, Some(1.0));
    }

    #[test]
    fn test_ontology_violation() {
        let mut constraints = OntologyConstraints::default();
        constraints.allowed_values.insert(
            "status".to_string(),
            vec![json!("active"), json!("inactive")],
        );

        let ctx = MetricContext::new("test.Type.v1", json!({"status": "unknown"}))
            .with_ontology(constraints);

        let computer = QomComputer::new().with_ontology(true);
        let result = computer.compute(&ctx);

        assert_eq!(result.metrics.ontology_adherence, Some(0.0));
        assert!(result.ontology_details.is_some());
        assert!(!result.ontology_details.unwrap().violations.is_empty());
    }
}
