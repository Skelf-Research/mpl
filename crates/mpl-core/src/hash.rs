//! Canonicalization and Semantic Hashing
//!
//! BLAKE3 hashes over canonical payloads detect meaning drift
//! across retries and multi-hop flows.

use serde_json::Value;

use crate::error::Result;

/// Canonicalize a JSON value for deterministic hashing
///
/// Steps:
/// 1. Sort all object keys recursively
/// 2. Remove null values (optional fields)
/// 3. Normalize numbers (no trailing zeros)
/// 4. Serialize with consistent formatting
pub fn canonicalize(value: &Value) -> Result<String> {
    let canonical = canonicalize_value(value);
    Ok(serde_json::to_string(&canonical)?)
}

/// Recursively canonicalize a JSON value
fn canonicalize_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            // Sort keys and recursively canonicalize values
            let mut sorted: Vec<_> = map
                .iter()
                .filter(|(_, v)| !v.is_null()) // Remove null values
                .map(|(k, v)| (k.clone(), canonicalize_value(v)))
                .collect();
            sorted.sort_by(|a, b| a.0.cmp(&b.0));
            Value::Object(sorted.into_iter().collect())
        }
        Value::Array(arr) => {
            // Recursively canonicalize array elements (preserve order)
            Value::Array(arr.iter().map(canonicalize_value).collect())
        }
        Value::Number(n) => {
            // Normalize numbers: convert to f64 and back to remove trailing zeros
            if let Some(f) = n.as_f64() {
                // Check if it's actually an integer
                if f.fract() == 0.0 && f.abs() < (i64::MAX as f64) {
                    Value::Number(serde_json::Number::from(f as i64))
                } else {
                    // Round to 6 decimal places for consistency
                    let rounded = (f * 1_000_000.0).round() / 1_000_000.0;
                    serde_json::Number::from_f64(rounded)
                        .map(Value::Number)
                        .unwrap_or_else(|| Value::Number(n.clone()))
                }
            } else {
                Value::Number(n.clone())
            }
        }
        Value::String(s) => {
            // Trim whitespace for consistency
            Value::String(s.trim().to_string())
        }
        // Bool and Null pass through unchanged
        other => other.clone(),
    }
}

/// Compute the semantic hash of a JSON value
///
/// Returns a BLAKE3 hash prefixed with "b3:" for identification
pub fn semantic_hash(value: &Value) -> Result<String> {
    let canonical = canonicalize(value)?;
    let hash = blake3::hash(canonical.as_bytes());
    Ok(format!("b3:{}", hash.to_hex()))
}

/// Compute semantic hash from a canonical string (already canonicalized)
pub fn hash_canonical(canonical: &str) -> String {
    let hash = blake3::hash(canonical.as_bytes());
    format!("b3:{}", hash.to_hex())
}

/// Verify that a payload matches its declared semantic hash
pub fn verify_hash(value: &Value, expected_hash: &str) -> Result<bool> {
    let actual = semantic_hash(value)?;
    Ok(actual == expected_hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_canonicalize_sorts_keys() {
        let input = json!({
            "z": 1,
            "a": 2,
            "m": 3
        });
        let canonical = canonicalize(&input).unwrap();
        assert_eq!(canonical, r#"{"a":2,"m":3,"z":1}"#);
    }

    #[test]
    fn test_canonicalize_removes_nulls() {
        let input = json!({
            "a": 1,
            "b": null,
            "c": 3
        });
        let canonical = canonicalize(&input).unwrap();
        assert_eq!(canonical, r#"{"a":1,"c":3}"#);
    }

    #[test]
    fn test_canonicalize_nested() {
        let input = json!({
            "outer": {
                "z": 1,
                "a": 2
            },
            "array": [{"b": 2, "a": 1}]
        });
        let canonical = canonicalize(&input).unwrap();
        assert_eq!(
            canonical,
            r#"{"array":[{"a":1,"b":2}],"outer":{"a":2,"z":1}}"#
        );
    }

    #[test]
    fn test_semantic_hash_deterministic() {
        let input = json!({
            "b": 2,
            "a": 1
        });
        let hash1 = semantic_hash(&input).unwrap();
        let hash2 = semantic_hash(&input).unwrap();
        assert_eq!(hash1, hash2);
        assert!(hash1.starts_with("b3:"));
    }

    #[test]
    fn test_semantic_hash_different_order_same_hash() {
        let input1 = json!({"a": 1, "b": 2});
        let input2 = json!({"b": 2, "a": 1});
        let hash1 = semantic_hash(&input1).unwrap();
        let hash2 = semantic_hash(&input2).unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_verify_hash() {
        let input = json!({"test": "value"});
        let hash = semantic_hash(&input).unwrap();
        assert!(verify_hash(&input, &hash).unwrap());
        assert!(!verify_hash(&input, "b3:wrong").unwrap());
    }
}
