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

// MplError is intentionally a wide enum with structured variants (e.g. QomBreach
// carries breach details, SchemaFidelity carries violations). The Err size lint
// is aesthetic; boxing would require refactoring every call-site signature for
// no observable benefit on this code path. Revisit if profiling shows it.
#![allow(clippy::result_large_err)]

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
pub mod util;
pub mod validation;

pub use assertions::{
    Assertion, AssertionEvaluator, AssertionResult, AssertionSet, AssertionSetResult,
    AssertionSeverity, EvaluationContext,
};
pub use envelope::MplEnvelope;
pub use error::{MplError, MplErrorCode};
pub use handshake::{ClientHello, ServerSelect};
pub use hash::{canonicalize, semantic_hash};
pub use metrics::{MetricComputeResult, MetricContext, QomComputer, TocMethod, TocResult};
pub use policy::{PolicyContext, PolicyDecision, PolicyEngine};
pub use qom::{QomProfile, QomReport};
pub use stype::SType;
pub use validation::SchemaValidator;

/// MPL protocol version
pub const MPL_VERSION: &str = "0.1.0";

/// Re-export commonly used types
pub mod prelude {
    pub use crate::assertions::{
        Assertion, AssertionEvaluator, AssertionResult, AssertionSet, AssertionSetResult,
        AssertionSeverity, EvaluationContext,
    };
    pub use crate::envelope::MplEnvelope;
    pub use crate::error::{MplError, MplErrorCode, Result};
    pub use crate::handshake::{ClientHello, ServerSelect};
    pub use crate::hash::{canonicalize, semantic_hash};
    pub use crate::metrics::{
        MetricComputeResult, MetricContext, QomComputer, TocMethod, TocResult,
    };
    pub use crate::policy::{Operation, Policy, PolicyContext, PolicyDecision, PolicyEngine};
    pub use crate::qom::{QomMetrics, QomProfile, QomReport};
    pub use crate::stype::SType;
    pub use crate::validation::SchemaValidator;
}
