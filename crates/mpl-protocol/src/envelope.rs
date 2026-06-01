//! MPL Envelope
//!
//! The semantic wrapper around payloads transmitted over MCP/A2A.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::hash::semantic_hash;
use crate::qom::QomReport;
use crate::stype::SType;

/// MPL Envelope - the core message wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MplEnvelope {
    /// Unique message identifier
    pub id: String,

    /// Semantic type of the payload
    pub stype: String,

    /// The actual payload data
    pub payload: serde_json::Value,

    /// Semantic type for arguments (for tool calls)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args_stype: Option<String>,

    /// QoM profile used for validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,

    /// Semantic hash of the canonical payload
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sem_hash: Option<String>,

    /// Provenance metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,

    /// QoM evaluation report (typically on responses)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qom_report: Option<QomReport>,

    /// Optional feature flags
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,

    /// Timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
}

impl MplEnvelope {
    /// Create a new envelope with a random ID
    pub fn new(stype: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            stype: stype.into(),
            payload,
            args_stype: None,
            profile: None,
            sem_hash: None,
            provenance: None,
            qom_report: None,
            features: Vec::new(),
            timestamp: Some(Utc::now()),
        }
    }

    /// Create an envelope from an SType
    pub fn from_stype(stype: &SType, payload: serde_json::Value) -> Self {
        Self::new(stype.id(), payload)
    }

    /// Set the args SType (for tool calls)
    pub fn with_args_stype(mut self, args_stype: impl Into<String>) -> Self {
        self.args_stype = Some(args_stype.into());
        self
    }

    /// Set the QoM profile
    pub fn with_profile(mut self, profile: impl Into<String>) -> Self {
        self.profile = Some(profile.into());
        self
    }

    /// Set provenance metadata
    pub fn with_provenance(mut self, provenance: Provenance) -> Self {
        self.provenance = Some(provenance);
        self
    }

    /// Add feature flags
    pub fn with_features(mut self, features: Vec<String>) -> Self {
        self.features = features;
        self
    }

    /// Compute and set the semantic hash
    pub fn compute_hash(&mut self) -> crate::error::Result<()> {
        self.sem_hash = Some(semantic_hash(&self.payload)?);
        Ok(())
    }

    /// Verify the semantic hash matches the payload
    pub fn verify_hash(&self) -> crate::error::Result<bool> {
        match &self.sem_hash {
            Some(expected) => {
                let actual = semantic_hash(&self.payload)?;
                Ok(&actual == expected)
            }
            None => Ok(true), // No hash to verify
        }
    }

    /// Attach a QoM report
    pub fn with_qom_report(mut self, report: QomReport) -> Self {
        self.qom_report = Some(report);
        self
    }

    /// Parse the SType field into a structured SType
    pub fn parsed_stype(&self) -> crate::error::Result<SType> {
        SType::parse(&self.stype)
    }
}

/// Provenance metadata tracking origin and transformation chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    /// Intent reference (SType or action identifier)
    pub intent: String,

    /// References to input context
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs_ref: Vec<String>,

    /// Consent receipt reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consent_ref: Option<String>,

    /// Policy reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_ref: Option<String>,

    /// Agent/model that produced this payload
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,

    /// Parent envelope ID (for tracing)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,

    /// Optional signature over semantic hash
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,

    /// Artifacts/sources for groundedness checks
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<Artifact>,
}

impl Provenance {
    /// Create new provenance with an intent
    pub fn new(intent: impl Into<String>) -> Self {
        Self {
            intent: intent.into(),
            inputs_ref: Vec::new(),
            consent_ref: None,
            policy_ref: None,
            agent: None,
            parent_id: None,
            signature: None,
            artifacts: Vec::new(),
        }
    }

    /// Add input references
    pub fn with_inputs(mut self, inputs: Vec<String>) -> Self {
        self.inputs_ref = inputs;
        self
    }

    /// Set consent reference
    pub fn with_consent(mut self, consent_ref: impl Into<String>) -> Self {
        self.consent_ref = Some(consent_ref.into());
        self
    }

    /// Set policy reference
    pub fn with_policy(mut self, policy_ref: impl Into<String>) -> Self {
        self.policy_ref = Some(policy_ref.into());
        self
    }

    /// Set agent identifier
    pub fn with_agent(mut self, agent: impl Into<String>) -> Self {
        self.agent = Some(agent.into());
        self
    }
}

/// Artifact for provenance (citations, sources)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    /// Reference identifier
    #[serde(rename = "ref")]
    pub reference: String,

    /// Type of artifact (document, api, database, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_type: Option<String>,

    /// Content or excerpt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// URL if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_envelope_creation() {
        let envelope = MplEnvelope::new(
            "org.calendar.Event.v1",
            json!({
                "title": "Meeting",
                "start": "2025-01-01T10:00:00Z"
            }),
        );

        assert!(!envelope.id.is_empty());
        assert_eq!(envelope.stype, "org.calendar.Event.v1");
        assert!(envelope.timestamp.is_some());
    }

    #[test]
    fn test_envelope_with_hash() {
        let mut envelope = MplEnvelope::new("org.test.Test.v1", json!({"key": "value"}));
        envelope.compute_hash().unwrap();

        assert!(envelope.sem_hash.is_some());
        assert!(envelope.sem_hash.as_ref().unwrap().starts_with("b3:"));
        assert!(envelope.verify_hash().unwrap());
    }

    #[test]
    fn test_envelope_serialization() {
        let envelope = MplEnvelope::new("org.test.Test.v1", json!({"test": true}))
            .with_profile("qom-basic")
            .with_provenance(Provenance::new("test.action.v1"));

        let json = serde_json::to_string(&envelope).unwrap();
        let deserialized: MplEnvelope = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.stype, envelope.stype);
        assert_eq!(deserialized.profile, envelope.profile);
    }
}
