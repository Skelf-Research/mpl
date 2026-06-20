//! MPL Error Taxonomy
//!
//! Typed errors that distinguish semantic failures from transport issues.
//! Each error includes hints for remediation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Result type for MPL operations
pub type Result<T> = std::result::Result<T, MplError>;

/// MPL error codes following the protocol specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING-KEBAB-CASE")]
pub enum MplErrorCode {
    /// QoM metric(s) failed to meet negotiated thresholds
    EQomBreach,
    /// Payload failed JSON Schema validation
    ESchemaFidelity,
    /// Tool arguments could not be coerced to declared args_stype
    EToolArgCoercion,
    /// Request violated negotiated policy
    EPolicyDenied,
    /// Referenced SType not found in registry
    EUnknownStype,
    /// Referenced tool not available
    EUnknownTool,
    /// Handshake failed - no compatible capability set
    ENegotiationIncompatible,
    /// Tool outcome verification failed
    EToolOutcomeIncorrect,
    /// Semantic hash mismatch detected
    ESemanticHashMismatch,
    /// Internal error
    EInternal,
}

impl MplErrorCode {
    /// Get the string representation of the error code
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::EQomBreach => "E-QOM-BREACH",
            Self::ESchemaFidelity => "E-SCHEMA-FIDELITY",
            Self::EToolArgCoercion => "E-TOOL-ARG-COERCION",
            Self::EPolicyDenied => "E-POLICY-DENIED",
            Self::EUnknownStype => "E-UNKNOWN-STYPE",
            Self::EUnknownTool => "E-UNKNOWN-TOOL",
            Self::ENegotiationIncompatible => "E-NEGOTIATION-INCOMPATIBLE",
            Self::EToolOutcomeIncorrect => "E-TOOL-OUTCOME-INCORRECT",
            Self::ESemanticHashMismatch => "E-SEMANTIC-HASH-MISMATCH",
            Self::EInternal => "E-INTERNAL",
        }
    }
}

impl std::fmt::Display for MplErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// MPL Error type
#[derive(Debug, Error)]
pub enum MplError {
    #[error("QoM breach: {message}")]
    QomBreach {
        message: String,
        metrics: HashMap<String, f64>,
        thresholds: HashMap<String, f64>,
        hints: Vec<String>,
    },

    #[error("Schema validation failed: {message}")]
    SchemaFidelity {
        message: String,
        stype: String,
        errors: Vec<SchemaError>,
        hints: Vec<String>,
    },

    #[error("Tool argument coercion failed: {message}")]
    ToolArgCoercion {
        message: String,
        tool_id: String,
        expected_stype: String,
        hints: Vec<String>,
    },

    #[error("Policy denied: {message}")]
    PolicyDenied {
        message: String,
        policy_ref: String,
        hints: Vec<String>,
    },

    #[error("Unknown SType: {stype}")]
    UnknownStype {
        stype: String,
        suggestions: Vec<String>,
    },

    #[error("Unknown tool: {tool_id}")]
    UnknownTool {
        tool_id: String,
        available: Vec<String>,
    },

    #[error("Negotiation incompatible: {message}")]
    NegotiationIncompatible {
        message: String,
        client_capabilities: Vec<String>,
        server_capabilities: Vec<String>,
    },

    #[error("Invalid SType format: {stype} - {reason}")]
    InvalidSType { stype: String, reason: String },

    #[error("Semantic hash mismatch: expected {expected}, got {actual}")]
    SemanticHashMismatch { expected: String, actual: String },

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl MplError {
    /// Get the error code for this error
    pub fn code(&self) -> MplErrorCode {
        match self {
            Self::QomBreach { .. } => MplErrorCode::EQomBreach,
            Self::SchemaFidelity { .. } => MplErrorCode::ESchemaFidelity,
            Self::ToolArgCoercion { .. } => MplErrorCode::EToolArgCoercion,
            Self::PolicyDenied { .. } => MplErrorCode::EPolicyDenied,
            Self::UnknownStype { .. } | Self::InvalidSType { .. } => MplErrorCode::EUnknownStype,
            Self::UnknownTool { .. } => MplErrorCode::EUnknownTool,
            Self::NegotiationIncompatible { .. } => MplErrorCode::ENegotiationIncompatible,
            Self::SemanticHashMismatch { .. } => MplErrorCode::ESemanticHashMismatch,
            Self::Validation(_) | Self::Serialization(_) | Self::Io(_) | Self::Internal(_) => {
                MplErrorCode::EInternal
            }
        }
    }

    /// Get remediation hints for this error
    pub fn hints(&self) -> Vec<String> {
        match self {
            Self::QomBreach { hints, .. } => hints.clone(),
            Self::SchemaFidelity { hints, .. } => hints.clone(),
            Self::ToolArgCoercion { hints, .. } => hints.clone(),
            Self::PolicyDenied { hints, .. } => hints.clone(),
            Self::UnknownStype { suggestions, .. } => {
                if suggestions.is_empty() {
                    vec!["Register the SType in the registry or check for typos".to_string()]
                } else {
                    suggestions
                        .iter()
                        .map(|s| format!("Did you mean: {}", s))
                        .collect()
                }
            }
            Self::UnknownTool { available, .. } => {
                vec![format!("Available tools: {}", available.join(", "))]
            }
            Self::NegotiationIncompatible { .. } => {
                vec!["Check protocol versions and capability sets".to_string()]
            }
            Self::InvalidSType { .. } => {
                vec!["Format: namespace.domain.Name.vN (e.g., org.calendar.Event.v1)".to_string()]
            }
            Self::SemanticHashMismatch { .. } => {
                vec!["Payload may have been modified in transit".to_string()]
            }
            _ => vec![],
        }
    }
}

/// Individual schema validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaError {
    /// JSON path to the error (e.g., "/start")
    pub path: String,
    /// Error message
    pub message: String,
    /// Expected type/value
    pub expected: Option<String>,
    /// Actual type/value found
    pub actual: Option<String>,
}

/// Structured error response for wire format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MplErrorResponse {
    pub code: String,
    pub message: String,
    pub hints: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl From<&MplError> for MplErrorResponse {
    fn from(err: &MplError) -> Self {
        let details = match err {
            MplError::QomBreach {
                metrics,
                thresholds,
                ..
            } => Some(serde_json::json!({
                "metrics": metrics,
                "thresholds": thresholds,
            })),
            MplError::SchemaFidelity { stype, errors, .. } => Some(serde_json::json!({
                "stype": stype,
                "errors": errors,
            })),
            MplError::UnknownStype { stype, suggestions } => Some(serde_json::json!({
                "stype": stype,
                "suggestions": suggestions,
            })),
            MplError::NegotiationIncompatible {
                client_capabilities,
                server_capabilities,
                ..
            } => Some(serde_json::json!({
                "client_capabilities": client_capabilities,
                "server_capabilities": server_capabilities,
            })),
            MplError::InvalidSType { stype, reason } => Some(serde_json::json!({
                "stype": stype,
                "reason": reason,
            })),
            _ => None,
        };

        Self {
            code: err.code().as_str().to_string(),
            message: err.to_string(),
            hints: err.hints(),
            details,
        }
    }
}

/// Builder for constructing detailed errors with context
pub struct ErrorBuilder {
    context: Vec<(String, String)>,
}

impl ErrorBuilder {
    pub fn new() -> Self {
        Self {
            context: Vec::new(),
        }
    }

    /// Add context to the error
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.push((key.into(), value.into()));
        self
    }

    /// Build a validation error with context
    pub fn validation_error(self, message: impl Into<String>) -> MplError {
        let msg = message.into();
        let context_str = if self.context.is_empty() {
            String::new()
        } else {
            let pairs: Vec<String> = self
                .context
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            format!(" [{}]", pairs.join(", "))
        };
        MplError::Validation(format!("{}{}", msg, context_str))
    }

    /// Build an internal error with context
    pub fn internal_error(self, message: impl Into<String>) -> MplError {
        let msg = message.into();
        let context_str = if self.context.is_empty() {
            String::new()
        } else {
            let pairs: Vec<String> = self
                .context
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            format!(" [{}]", pairs.join(", "))
        };
        MplError::Internal(format!("{}{}", msg, context_str))
    }
}

impl Default for ErrorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to create a schema fidelity error with detailed context
pub fn schema_error(stype: &str, errors: Vec<SchemaError>) -> MplError {
    let error_summary = if errors.len() == 1 {
        errors[0].message.clone()
    } else {
        format!("{} validation errors", errors.len())
    };

    let hints: Vec<String> = errors
        .iter()
        .take(3)
        .map(|e| {
            if let (Some(expected), Some(actual)) = (&e.expected, &e.actual) {
                format!("At {}: expected {}, got {}", e.path, expected, actual)
            } else {
                format!("At {}: {}", e.path, e.message)
            }
        })
        .collect();

    MplError::SchemaFidelity {
        message: error_summary,
        stype: stype.to_string(),
        errors,
        hints,
    }
}

/// Helper to create a QoM breach error with metrics context
pub fn qom_breach_error(
    profile: &str,
    metrics: HashMap<String, f64>,
    thresholds: HashMap<String, f64>,
) -> MplError {
    let failed_metrics: Vec<String> = thresholds
        .iter()
        .filter_map(|(name, threshold)| {
            let value = metrics.get(name).unwrap_or(&0.0);
            if value < threshold {
                Some(format!("{}: {} < {} (threshold)", name, value, threshold))
            } else {
                None
            }
        })
        .collect();

    let message = format!(
        "Profile '{}' requirements not met: {}",
        profile,
        failed_metrics.join("; ")
    );

    let hints = vec![
        "Check instruction compliance by validating agent behavior".to_string(),
        "Verify schema fidelity by ensuring payload matches schema".to_string(),
        format!("Consider using a less strict profile than '{}'", profile),
    ];

    MplError::QomBreach {
        message,
        metrics,
        thresholds,
        hints,
    }
}
