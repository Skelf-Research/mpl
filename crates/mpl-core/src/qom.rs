//! Quality of Meaning (QoM)
//!
//! Framework for measuring and enforcing semantic quality through
//! observable metrics, negotiated profiles, and actionable breach detection.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// QoM Profile - configuration defining metric thresholds and policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QomProfile {
    /// Profile name (e.g., "qom-strict-argcheck", "qom-basic")
    pub name: String,

    /// Metric thresholds
    pub metrics: QomMetricThresholds,

    /// Retry policy on failure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_policy: Option<RetryPolicy>,

    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl QomProfile {
    /// Create a basic profile with Schema Fidelity only
    pub fn basic() -> Self {
        Self {
            name: "qom-basic".to_string(),
            metrics: QomMetricThresholds {
                schema_fidelity: Some(MetricThreshold::required(1.0)),
                instruction_compliance: None,
                groundedness: None,
                determinism_jitter: None,
                ontology_adherence: None,
                tool_outcome_correctness: None,
            },
            retry_policy: None,
            description: Some("Basic validation: Schema Fidelity only".to_string()),
        }
    }

    /// Create a strict profile with SF and IC
    pub fn strict_argcheck() -> Self {
        Self {
            name: "qom-strict-argcheck".to_string(),
            metrics: QomMetricThresholds {
                schema_fidelity: Some(MetricThreshold::required(1.0)),
                instruction_compliance: Some(MetricThreshold::required(0.97)),
                groundedness: None,
                determinism_jitter: None,
                ontology_adherence: None,
                tool_outcome_correctness: None,
            },
            retry_policy: Some(RetryPolicy {
                max_retries: 1,
                degrade_to: Some("qom-basic".to_string()),
                on_failure: FailureAction::Escalate,
            }),
            description: Some("Strict validation: SF=1.0, IC>=0.97".to_string()),
        }
    }

    /// Create an outcome-focused profile with SF and TOC
    pub fn outcome() -> Self {
        Self {
            name: "qom-outcome".to_string(),
            metrics: QomMetricThresholds {
                schema_fidelity: Some(MetricThreshold::required(1.0)),
                instruction_compliance: None,
                groundedness: None,
                determinism_jitter: None,
                ontology_adherence: None,
                tool_outcome_correctness: Some(MetricThreshold::required(0.9)),
            },
            retry_policy: Some(RetryPolicy {
                max_retries: 2,
                degrade_to: Some("qom-basic".to_string()),
                on_failure: FailureAction::Error,
            }),
            description: Some("Outcome validation: SF=1.0, TOC>=0.9".to_string()),
        }
    }

    /// Create a comprehensive profile with all major metrics
    pub fn comprehensive() -> Self {
        Self {
            name: "qom-comprehensive".to_string(),
            metrics: QomMetricThresholds {
                schema_fidelity: Some(MetricThreshold::required(1.0)),
                instruction_compliance: Some(MetricThreshold::required(0.95)),
                groundedness: Some(MetricThreshold::sampled(0.8, 0.5)),
                determinism_jitter: Some(MetricThreshold::sampled(0.9, 0.3)),
                ontology_adherence: Some(MetricThreshold::required(0.95)),
                tool_outcome_correctness: Some(MetricThreshold::required(0.9)),
            },
            retry_policy: Some(RetryPolicy {
                max_retries: 2,
                degrade_to: Some("qom-strict-argcheck".to_string()),
                on_failure: FailureAction::Escalate,
            }),
            description: Some("Comprehensive validation: all metrics enforced".to_string()),
        }
    }

    /// Evaluate metrics against this profile
    pub fn evaluate(&self, metrics: &QomMetrics) -> QomEvaluation {
        let mut passed = true;
        let mut failures = Vec::new();

        // Schema Fidelity (mandatory)
        if let Some(threshold) = &self.metrics.schema_fidelity {
            if metrics.schema_fidelity < threshold.min {
                passed = false;
                failures.push(MetricFailure {
                    metric: "schema_fidelity".to_string(),
                    actual: metrics.schema_fidelity,
                    threshold: threshold.min,
                });
            }
        }

        // Instruction Compliance
        if let Some(threshold) = &self.metrics.instruction_compliance {
            if let Some(ic) = metrics.instruction_compliance {
                if ic < threshold.min {
                    passed = false;
                    failures.push(MetricFailure {
                        metric: "instruction_compliance".to_string(),
                        actual: ic,
                        threshold: threshold.min,
                    });
                }
            } else if threshold.min > 0.0 {
                // IC required but not provided
                passed = false;
                failures.push(MetricFailure {
                    metric: "instruction_compliance".to_string(),
                    actual: 0.0,
                    threshold: threshold.min,
                });
            }
        }

        // Groundedness (Phase 2+)
        if let Some(threshold) = &self.metrics.groundedness {
            if let Some(g) = metrics.groundedness {
                if g < threshold.min {
                    passed = false;
                    failures.push(MetricFailure {
                        metric: "groundedness".to_string(),
                        actual: g,
                        threshold: threshold.min,
                    });
                }
            }
        }

        // Determinism Jitter (Phase 2+)
        if let Some(threshold) = &self.metrics.determinism_jitter {
            if let Some(dj) = metrics.determinism_jitter {
                if dj < threshold.min {
                    passed = false;
                    failures.push(MetricFailure {
                        metric: "determinism_jitter".to_string(),
                        actual: dj,
                        threshold: threshold.min,
                    });
                }
            }
        }

        // Ontology Adherence
        if let Some(threshold) = &self.metrics.ontology_adherence {
            if let Some(oa) = metrics.ontology_adherence {
                if oa < threshold.min {
                    passed = false;
                    failures.push(MetricFailure {
                        metric: "ontology_adherence".to_string(),
                        actual: oa,
                        threshold: threshold.min,
                    });
                }
            }
        }

        // Tool Outcome Correctness
        if let Some(threshold) = &self.metrics.tool_outcome_correctness {
            if let Some(toc) = metrics.tool_outcome_correctness {
                if toc < threshold.min {
                    passed = false;
                    failures.push(MetricFailure {
                        metric: "tool_outcome_correctness".to_string(),
                        actual: toc,
                        threshold: threshold.min,
                    });
                }
            } else if threshold.min > 0.0 {
                // TOC required but not provided
                passed = false;
                failures.push(MetricFailure {
                    metric: "tool_outcome_correctness".to_string(),
                    actual: 0.0,
                    threshold: threshold.min,
                });
            }
        }

        QomEvaluation {
            meets_profile: passed,
            profile: self.name.clone(),
            failures,
        }
    }
}

/// Metric thresholds for a QoM profile
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QomMetricThresholds {
    /// Schema Fidelity: payload conforms to SType schema (mandatory, target: 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_fidelity: Option<MetricThreshold>,

    /// Instruction Compliance: adherence to assertions/constraints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instruction_compliance: Option<MetricThreshold>,

    /// Groundedness: claims supported by citations (Phase 2+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groundedness: Option<MetricThreshold>,

    /// Determinism under Jitter: output stability (Phase 2+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub determinism_jitter: Option<MetricThreshold>,

    /// Ontology Adherence: domain constraint conformance
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ontology_adherence: Option<MetricThreshold>,

    /// Tool Outcome Correctness: side effects match expectations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_outcome_correctness: Option<MetricThreshold>,
}

/// Threshold configuration for a single metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricThreshold {
    /// Minimum acceptable value
    pub min: f64,

    /// Sampling rate (0.0-1.0) for expensive metrics
    #[serde(default = "default_sample_rate")]
    pub sample_rate: f64,
}

fn default_sample_rate() -> f64 {
    1.0
}

impl MetricThreshold {
    /// Create a required threshold (100% sampling)
    pub fn required(min: f64) -> Self {
        Self {
            min,
            sample_rate: 1.0,
        }
    }

    /// Create a sampled threshold
    pub fn sampled(min: f64, sample_rate: f64) -> Self {
        Self { min, sample_rate }
    }
}

/// Computed QoM metrics for a payload
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QomMetrics {
    /// Schema Fidelity score (0.0-1.0)
    pub schema_fidelity: f64,

    /// Instruction Compliance score (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instruction_compliance: Option<f64>,

    /// Groundedness score (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groundedness: Option<f64>,

    /// Determinism Jitter score (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub determinism_jitter: Option<f64>,

    /// Ontology Adherence score (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ontology_adherence: Option<f64>,

    /// Tool Outcome Correctness (pass/fail as 1.0/0.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_outcome_correctness: Option<f64>,
}

impl QomMetrics {
    /// Create metrics with perfect schema fidelity
    pub fn schema_valid() -> Self {
        Self {
            schema_fidelity: 1.0,
            ..Default::default()
        }
    }

    /// Create metrics indicating schema failure
    pub fn schema_invalid() -> Self {
        Self {
            schema_fidelity: 0.0,
            ..Default::default()
        }
    }

    /// Set instruction compliance score
    pub fn with_instruction_compliance(mut self, score: f64) -> Self {
        self.instruction_compliance = Some(score);
        self
    }

    /// Set tool outcome correctness score
    pub fn with_tool_outcome_correctness(mut self, score: f64) -> Self {
        self.tool_outcome_correctness = Some(score);
        self
    }

    /// Set groundedness score
    pub fn with_groundedness(mut self, score: f64) -> Self {
        self.groundedness = Some(score);
        self
    }

    /// Set ontology adherence score
    pub fn with_ontology_adherence(mut self, score: f64) -> Self {
        self.ontology_adherence = Some(score);
        self
    }

    /// Set determinism jitter score
    pub fn with_determinism_jitter(mut self, score: f64) -> Self {
        self.determinism_jitter = Some(score);
        self
    }

    /// Convert to a HashMap for reporting
    pub fn to_map(&self) -> HashMap<String, f64> {
        let mut map = HashMap::new();
        map.insert("schema_fidelity".to_string(), self.schema_fidelity);
        if let Some(ic) = self.instruction_compliance {
            map.insert("instruction_compliance".to_string(), ic);
        }
        if let Some(g) = self.groundedness {
            map.insert("groundedness".to_string(), g);
        }
        if let Some(dj) = self.determinism_jitter {
            map.insert("determinism_jitter".to_string(), dj);
        }
        if let Some(oa) = self.ontology_adherence {
            map.insert("ontology_adherence".to_string(), oa);
        }
        if let Some(toc) = self.tool_outcome_correctness {
            map.insert("tool_outcome_correctness".to_string(), toc);
        }
        map
    }
}

/// QoM evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QomEvaluation {
    /// Whether the payload meets the profile
    pub meets_profile: bool,
    /// Profile name evaluated against
    pub profile: String,
    /// Failed metrics
    pub failures: Vec<MetricFailure>,
}

/// Individual metric failure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricFailure {
    pub metric: String,
    pub actual: f64,
    pub threshold: f64,
}

/// QoM Report attached to responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QomReport {
    /// Whether payload meets negotiated profile
    pub meets_profile: bool,

    /// Profile evaluated against
    pub profile: String,

    /// Computed metric scores
    pub metrics: QomMetrics,

    /// Evaluation details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evaluation: Option<QomEvaluation>,

    /// References to detailed artifacts
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifact_refs: Vec<String>,

    /// Hints for remediation if failed
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hints: Vec<String>,
}

impl QomReport {
    /// Create a passing report
    pub fn pass(profile: impl Into<String>, metrics: QomMetrics) -> Self {
        Self {
            meets_profile: true,
            profile: profile.into(),
            metrics,
            evaluation: None,
            artifact_refs: Vec::new(),
            hints: Vec::new(),
        }
    }

    /// Create a failing report
    pub fn fail(profile: impl Into<String>, metrics: QomMetrics, evaluation: QomEvaluation) -> Self {
        let hints = evaluation
            .failures
            .iter()
            .map(|f| {
                format!(
                    "{}: got {:.2}, expected >= {:.2}",
                    f.metric, f.actual, f.threshold
                )
            })
            .collect();

        Self {
            meets_profile: false,
            profile: profile.into(),
            metrics,
            evaluation: Some(evaluation),
            artifact_refs: Vec::new(),
            hints,
        }
    }
}

/// Retry policy for QoM failures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum retry attempts
    pub max_retries: u32,

    /// Profile to degrade to on failure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub degrade_to: Option<String>,

    /// Action on final failure
    pub on_failure: FailureAction,
}

/// Action to take on final QoM failure
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureAction {
    /// Escalate to human/supervisor
    Escalate,
    /// Return error to caller
    Error,
    /// Log and continue (best effort)
    Warn,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_profile() {
        let profile = QomProfile::basic();
        let metrics = QomMetrics::schema_valid();
        let eval = profile.evaluate(&metrics);
        assert!(eval.meets_profile);
    }

    #[test]
    fn test_strict_profile_pass() {
        let profile = QomProfile::strict_argcheck();
        let metrics = QomMetrics::schema_valid().with_instruction_compliance(0.98);
        let eval = profile.evaluate(&metrics);
        assert!(eval.meets_profile);
    }

    #[test]
    fn test_strict_profile_fail() {
        let profile = QomProfile::strict_argcheck();
        let metrics = QomMetrics::schema_valid().with_instruction_compliance(0.90);
        let eval = profile.evaluate(&metrics);
        assert!(!eval.meets_profile);
        assert_eq!(eval.failures.len(), 1);
        assert_eq!(eval.failures[0].metric, "instruction_compliance");
    }

    #[test]
    fn test_schema_failure() {
        let profile = QomProfile::basic();
        let metrics = QomMetrics::schema_invalid();
        let eval = profile.evaluate(&metrics);
        assert!(!eval.meets_profile);
    }

    #[test]
    fn test_outcome_profile_pass() {
        let profile = QomProfile::outcome();
        let metrics = QomMetrics::schema_valid().with_tool_outcome_correctness(0.95);
        let eval = profile.evaluate(&metrics);
        assert!(eval.meets_profile);
    }

    #[test]
    fn test_outcome_profile_fail() {
        let profile = QomProfile::outcome();
        let metrics = QomMetrics::schema_valid().with_tool_outcome_correctness(0.8);
        let eval = profile.evaluate(&metrics);
        assert!(!eval.meets_profile);
        assert_eq!(eval.failures.len(), 1);
        assert_eq!(eval.failures[0].metric, "tool_outcome_correctness");
    }

    #[test]
    fn test_comprehensive_profile() {
        let profile = QomProfile::comprehensive();
        let metrics = QomMetrics::schema_valid()
            .with_instruction_compliance(0.96)
            .with_groundedness(0.85)
            .with_ontology_adherence(0.98)
            .with_tool_outcome_correctness(0.92);
        let eval = profile.evaluate(&metrics);
        assert!(eval.meets_profile);
    }
}
