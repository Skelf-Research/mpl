//! MPL Core Library
//!
//! Core primitives for the Meaning Protocol Layer:
//! - Semantic Types (STypes) and schemas
//! - MPL Envelope structure
//! - AI-ALPN handshake messages
//! - QoM profiles and evaluation
//! - CEL-based assertions for Instruction Compliance
//! - Policy engine for rule-based enforcement
//! - Canonicalization and semantic hashing
//! - Error taxonomy

pub mod assertions;
pub mod determinism;
pub mod envelope;
pub mod error;
pub mod groundedness;
pub mod handshake;
pub mod hash;
pub mod metrics;
pub mod ontology;
pub mod policy;
pub mod qom;
pub mod stype;
pub mod validation;

pub use assertions::{Assertion, AssertionSet, AssertionEvaluator, AssertionResult, AssertionSetResult, AssertionSeverity, EvaluationContext};
pub use envelope::MplEnvelope;
pub use error::{MplError, MplErrorCode};
pub use handshake::{ClientHello, ServerSelect};
pub use hash::{canonicalize, semantic_hash};
pub use metrics::{QomComputer, MetricContext, MetricComputeResult, TocResult, TocMethod};
pub use policy::{PolicyEngine, PolicyContext, PolicyDecision};
pub use qom::{QomProfile, QomReport};
pub use stype::SType;
pub use validation::SchemaValidator;

/// MPL protocol version
pub const MPL_VERSION: &str = "0.1.0";

/// Re-export commonly used types
pub mod prelude {
    pub use crate::assertions::{Assertion, AssertionSet, AssertionEvaluator, AssertionResult, AssertionSetResult, AssertionSeverity, EvaluationContext};
    pub use crate::envelope::MplEnvelope;
    pub use crate::error::{MplError, MplErrorCode, Result};
    pub use crate::handshake::{ClientHello, ServerSelect};
    pub use crate::hash::{canonicalize, semantic_hash};
    pub use crate::metrics::{QomComputer, MetricContext, MetricComputeResult, TocResult, TocMethod};
    pub use crate::policy::{PolicyEngine, PolicyContext, PolicyDecision, Policy, Operation};
    pub use crate::qom::{QomMetrics, QomProfile, QomReport};
    pub use crate::stype::SType;
    pub use crate::validation::SchemaValidator;
}
