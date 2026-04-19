//! MPL Core Library
//!
//! Core primitives for the Meaning Protocol Layer:
//! - Semantic Types (STypes) and schemas
//! - MPL Envelope structure
//! - AI-ALPN handshake messages
//! - QoM profiles and evaluation
//! - Policy engine for rule-based enforcement
//! - Canonicalization and semantic hashing
//! - Error taxonomy

pub mod envelope;
pub mod error;
pub mod handshake;
pub mod hash;
pub mod policy;
pub mod qom;
pub mod stype;
pub mod validation;

pub use envelope::MplEnvelope;
pub use error::{MplError, MplErrorCode};
pub use handshake::{ClientHello, ServerSelect};
pub use hash::{canonicalize, semantic_hash};
pub use policy::{PolicyEngine, PolicyContext, PolicyDecision};
pub use qom::{QomProfile, QomReport};
pub use stype::SType;
pub use validation::SchemaValidator;

/// MPL protocol version
pub const MPL_VERSION: &str = "0.1.0";

/// Re-export commonly used types
pub mod prelude {
    pub use crate::envelope::MplEnvelope;
    pub use crate::error::{MplError, MplErrorCode, Result};
    pub use crate::handshake::{ClientHello, ServerSelect};
    pub use crate::hash::{canonicalize, semantic_hash};
    pub use crate::policy::{PolicyEngine, PolicyContext, PolicyDecision, Policy, Operation};
    pub use crate::qom::{QomMetrics, QomProfile, QomReport};
    pub use crate::stype::SType;
    pub use crate::validation::SchemaValidator;
}
