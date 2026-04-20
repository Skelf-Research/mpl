//! Determinism Jitter Detection
//!
//! Measures output stability across multiple runs of the same request.
//! This helps detect non-deterministic behavior in AI agents.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use tracing::debug;

/// Configuration for determinism checking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeterminismConfig {
    /// Number of responses to keep for comparison
    #[serde(default = "default_history_size")]
    pub history_size: usize,

    /// Minimum similarity for responses to be considered deterministic (0.0 - 1.0)
    #[serde(default = "default_similarity_threshold")]
    pub similarity_threshold: f64,

    /// Fields to ignore when comparing (e.g., timestamps)
    #[serde(default)]
    pub ignore_fields: Vec<String>,

    /// Whether to normalize whitespace before comparison
    #[serde(default = "default_true")]
    pub normalize_whitespace: bool,

    /// Whether to ignore field ordering in objects
    #[serde(default = "default_true")]
    pub ignore_field_order: bool,
}

fn default_history_size() -> usize {
    5
}

fn default_similarity_threshold() -> f64 {
    0.9
}

fn default_true() -> bool {
    true
}

impl Default for DeterminismConfig {
    fn default() -> Self {
        Self {
            history_size: default_history_size(),
            similarity_threshold: default_similarity_threshold(),
            ignore_fields: vec![
                "timestamp".to_string(),
                "created_at".to_string(),
                "updated_at".to_string(),
                "request_id".to_string(),
                "trace_id".to_string(),
            ],
            normalize_whitespace: true,
            ignore_field_order: true,
        }
    }
}

/// A field difference between two responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDifference {
    /// JSON path to the field
    pub path: String,

    /// Value in the reference response
    pub expected: serde_json::Value,

    /// Value in the current response
    pub actual: serde_json::Value,

    /// Type of difference
    pub diff_type: DifferenceType,
}

/// Types of differences
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DifferenceType {
    /// Value changed
    ValueChanged,
    /// Field added
    FieldAdded,
    /// Field removed
    FieldRemoved,
    /// Type changed
    TypeChanged,
    /// Array length changed
    ArrayLengthChanged,
}

/// Result of determinism check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeterminismResult {
    /// Similarity score (0.0 - 1.0)
    pub similarity: f64,

    /// Whether the response is considered deterministic
    pub is_deterministic: bool,

    /// List of differences found
    pub differences: Vec<FieldDifference>,

    /// Number of comparisons made
    pub comparison_count: usize,

    /// Average similarity across all comparisons
    pub average_similarity: f64,

    /// Jitter score (1 - similarity, higher = more jitter)
    pub jitter: f64,
}

/// Request signature for grouping responses
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestSignature {
    /// SType of the request
    pub stype: String,

    /// Canonical hash of the request payload
    pub payload_hash: String,

    /// Tool name (if applicable)
    pub tool_name: Option<String>,
}

/// Determinism checker with response history
pub struct DeterminismChecker {
    config: DeterminismConfig,
    /// History of responses by request signature
    history: HashMap<RequestSignature, VecDeque<serde_json::Value>>,
}

impl Default for DeterminismChecker {
    fn default() -> Self {
        Self::new(DeterminismConfig::default())
    }
}

impl DeterminismChecker {
    /// Create a new determinism checker
    pub fn new(config: DeterminismConfig) -> Self {
        Self {
            config,
            history: HashMap::new(),
        }
    }

    /// Check determinism of a response and add it to history
    pub fn check_and_record(
        &mut self,
        signature: &RequestSignature,
        response: &serde_json::Value,
    ) -> DeterminismResult {
        let result = self.check(signature, response);

        // Normalize response before storing
        let normalized = self.normalize_value(response);
        let history_size = self.config.history_size;

        // Add to history
        let history = self
            .history
            .entry(signature.clone())
            .or_insert_with(VecDeque::new);

        history.push_back(normalized);

        // Keep only recent history
        while history.len() > history_size {
            history.pop_front();
        }

        result
    }

    /// Check determinism without recording
    pub fn check(
        &self,
        signature: &RequestSignature,
        response: &serde_json::Value,
    ) -> DeterminismResult {
        let history = match self.history.get(signature) {
            Some(h) if !h.is_empty() => h,
            _ => {
                // No history to compare against
                return DeterminismResult {
                    similarity: 1.0,
                    is_deterministic: true,
                    differences: vec![],
                    comparison_count: 0,
                    average_similarity: 1.0,
                    jitter: 0.0,
                };
            }
        };

        let normalized = self.normalize_value(response);
        let mut total_similarity = 0.0;
        let mut all_differences = Vec::new();

        for historical in history.iter() {
            let (similarity, differences) = self.compare_values(&normalized, historical, "");
            total_similarity += similarity;

            // Collect unique differences
            for diff in differences {
                if !all_differences.iter().any(|d: &FieldDifference| d.path == diff.path) {
                    all_differences.push(diff);
                }
            }
        }

        let comparison_count = history.len();
        let average_similarity = if comparison_count > 0 {
            total_similarity / comparison_count as f64
        } else {
            1.0
        };

        let is_deterministic = average_similarity >= self.config.similarity_threshold;
        let jitter = 1.0 - average_similarity;

        debug!(
            "Determinism check: similarity={:.3}, is_deterministic={}, differences={}",
            average_similarity,
            is_deterministic,
            all_differences.len()
        );

        DeterminismResult {
            similarity: average_similarity,
            is_deterministic,
            differences: all_differences,
            comparison_count,
            average_similarity,
            jitter,
        }
    }

    /// Normalize a value for comparison
    fn normalize_value(&self, value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(obj) => {
                let mut normalized = serde_json::Map::new();
                for (key, val) in obj {
                    // Skip ignored fields
                    if self.config.ignore_fields.contains(key) {
                        continue;
                    }
                    normalized.insert(key.clone(), self.normalize_value(val));
                }
                serde_json::Value::Object(normalized)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(|v| self.normalize_value(v)).collect())
            }
            serde_json::Value::String(s) => {
                if self.config.normalize_whitespace {
                    let normalized: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
                    serde_json::Value::String(normalized)
                } else {
                    value.clone()
                }
            }
            _ => value.clone(),
        }
    }

    /// Compare two values and return similarity + differences
    fn compare_values(
        &self,
        current: &serde_json::Value,
        reference: &serde_json::Value,
        path: &str,
    ) -> (f64, Vec<FieldDifference>) {
        let mut differences = Vec::new();

        match (current, reference) {
            (serde_json::Value::Object(curr_obj), serde_json::Value::Object(ref_obj)) => {
                let mut total_fields = 0;
                let mut matching_fields = 0;

                // Check fields in current
                for (key, curr_val) in curr_obj {
                    let field_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };

                    total_fields += 1;

                    if let Some(ref_val) = ref_obj.get(key) {
                        let (sim, mut diffs) = self.compare_values(curr_val, ref_val, &field_path);
                        if sim >= self.config.similarity_threshold {
                            matching_fields += 1;
                        }
                        differences.append(&mut diffs);
                    } else {
                        differences.push(FieldDifference {
                            path: field_path,
                            expected: serde_json::Value::Null,
                            actual: curr_val.clone(),
                            diff_type: DifferenceType::FieldAdded,
                        });
                    }
                }

                // Check for removed fields
                for key in ref_obj.keys() {
                    if !curr_obj.contains_key(key) {
                        let field_path = if path.is_empty() {
                            key.clone()
                        } else {
                            format!("{}.{}", path, key)
                        };
                        total_fields += 1;
                        differences.push(FieldDifference {
                            path: field_path,
                            expected: ref_obj.get(key).cloned().unwrap_or(serde_json::Value::Null),
                            actual: serde_json::Value::Null,
                            diff_type: DifferenceType::FieldRemoved,
                        });
                    }
                }

                let similarity = if total_fields > 0 {
                    matching_fields as f64 / total_fields as f64
                } else {
                    1.0
                };

                (similarity, differences)
            }
            (serde_json::Value::Array(curr_arr), serde_json::Value::Array(ref_arr)) => {
                if curr_arr.len() != ref_arr.len() {
                    differences.push(FieldDifference {
                        path: path.to_string(),
                        expected: serde_json::Value::Number(ref_arr.len().into()),
                        actual: serde_json::Value::Number(curr_arr.len().into()),
                        diff_type: DifferenceType::ArrayLengthChanged,
                    });
                }

                let mut total_similarity = 0.0;
                let min_len = curr_arr.len().min(ref_arr.len());

                for (i, (curr_item, ref_item)) in
                    curr_arr.iter().zip(ref_arr.iter()).enumerate()
                {
                    let item_path = format!("{}[{}]", path, i);
                    let (sim, mut diffs) = self.compare_values(curr_item, ref_item, &item_path);
                    total_similarity += sim;
                    differences.append(&mut diffs);
                }

                let max_len = curr_arr.len().max(ref_arr.len());
                let similarity = if max_len > 0 {
                    (total_similarity / min_len as f64) * (min_len as f64 / max_len as f64)
                } else {
                    1.0
                };

                (similarity, differences)
            }
            _ => {
                // Primitive comparison
                if current == reference {
                    (1.0, differences)
                } else {
                    // Check if types match
                    let diff_type = if std::mem::discriminant(current)
                        != std::mem::discriminant(reference)
                    {
                        DifferenceType::TypeChanged
                    } else {
                        DifferenceType::ValueChanged
                    };

                    differences.push(FieldDifference {
                        path: path.to_string(),
                        expected: reference.clone(),
                        actual: current.clone(),
                        diff_type,
                    });

                    // For strings, calculate partial similarity
                    if let (serde_json::Value::String(s1), serde_json::Value::String(s2)) =
                        (current, reference)
                    {
                        let similarity = string_similarity(s1, s2);
                        (similarity, differences)
                    } else {
                        (0.0, differences)
                    }
                }
            }
        }
    }

    /// Clear history for a specific signature
    pub fn clear_history(&mut self, signature: &RequestSignature) {
        self.history.remove(signature);
    }

    /// Clear all history
    pub fn clear_all_history(&mut self) {
        self.history.clear();
    }

    /// Get history size for a signature
    pub fn history_size(&self, signature: &RequestSignature) -> usize {
        self.history.get(signature).map(|h| h.len()).unwrap_or(0)
    }
}

/// Calculate string similarity using Jaccard index on words
fn string_similarity(s1: &str, s2: &str) -> f64 {
    let words1: std::collections::HashSet<&str> = s1.split_whitespace().collect();
    let words2: std::collections::HashSet<&str> = s2.split_whitespace().collect();

    let intersection = words1.intersection(&words2).count();
    let union = words1.union(&words2).count();

    if union == 0 {
        1.0
    } else {
        intersection as f64 / union as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_signature(stype: &str) -> RequestSignature {
        RequestSignature {
            stype: stype.to_string(),
            payload_hash: "test_hash".to_string(),
            tool_name: None,
        }
    }

    #[test]
    fn test_identical_responses() {
        let mut checker = DeterminismChecker::default();
        let sig = make_signature("test.Type.v1");

        let response = json!({"result": "hello", "count": 5});

        // First response - no history
        let result1 = checker.check_and_record(&sig, &response);
        assert!(result1.is_deterministic);
        assert_eq!(result1.comparison_count, 0);

        // Second identical response
        let result2 = checker.check_and_record(&sig, &response);
        assert!(result2.is_deterministic);
        assert_eq!(result2.similarity, 1.0);
        assert!(result2.differences.is_empty());
    }

    #[test]
    fn test_different_responses() {
        let mut checker = DeterminismChecker::default();
        let sig = make_signature("test.Type.v1");

        let response1 = json!({"result": "hello", "count": 5});
        let response2 = json!({"result": "world", "count": 10});

        checker.check_and_record(&sig, &response1);
        let result = checker.check_and_record(&sig, &response2);

        assert!(!result.is_deterministic);
        assert!(result.similarity < 1.0);
        assert!(!result.differences.is_empty());
    }

    #[test]
    fn test_ignored_fields() {
        let mut checker = DeterminismChecker::default();
        let sig = make_signature("test.Type.v1");

        let response1 = json!({"result": "hello", "timestamp": "2024-01-01T00:00:00Z"});
        let response2 = json!({"result": "hello", "timestamp": "2024-01-02T00:00:00Z"});

        checker.check_and_record(&sig, &response1);
        let result = checker.check_and_record(&sig, &response2);

        // Timestamp should be ignored
        assert!(result.is_deterministic);
        assert_eq!(result.similarity, 1.0);
    }

    #[test]
    fn test_whitespace_normalization() {
        let mut checker = DeterminismChecker::default();
        let sig = make_signature("test.Type.v1");

        let response1 = json!({"text": "hello   world"});
        let response2 = json!({"text": "hello world"});

        checker.check_and_record(&sig, &response1);
        let result = checker.check_and_record(&sig, &response2);

        // Whitespace should be normalized
        assert!(result.is_deterministic);
    }

    #[test]
    fn test_history_limit() {
        let mut checker = DeterminismChecker::new(DeterminismConfig {
            history_size: 3,
            ..Default::default()
        });
        let sig = make_signature("test.Type.v1");

        for i in 0..10 {
            checker.check_and_record(&sig, &json!({"count": i}));
        }

        // Should only keep last 3
        assert_eq!(checker.history_size(&sig), 3);
    }

    #[test]
    fn test_jitter_calculation() {
        let mut checker = DeterminismChecker::default();
        let sig = make_signature("test.Type.v1");

        checker.check_and_record(&sig, &json!({"value": 1}));
        let result = checker.check_and_record(&sig, &json!({"value": 2}));

        // Jitter should be 1 - similarity
        assert!((result.jitter - (1.0 - result.similarity)).abs() < 0.001);
    }
}
