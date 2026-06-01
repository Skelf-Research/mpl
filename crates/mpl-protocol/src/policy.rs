//! Policy Engine Lite
//!
//! Simple rule-based policy engine for enforcing SType usage rules,
//! access control, and semantic constraints.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::stype::SType;

/// Policy Engine for enforcing rules on SType operations
#[derive(Debug, Clone, Default)]
pub struct PolicyEngine {
    /// Registered policies
    policies: Vec<Policy>,
    /// Default QoM profile
    default_profile: Option<String>,
}

impl PolicyEngine {
    /// Create a new policy engine
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a policy
    pub fn add_policy(&mut self, policy: Policy) {
        self.policies.push(policy);
    }

    /// Set default QoM profile
    pub fn set_default_profile(&mut self, profile: impl Into<String>) {
        self.default_profile = Some(profile.into());
    }

    /// Evaluate policies for an SType operation
    pub fn evaluate(&self, context: &PolicyContext) -> PolicyDecision {
        let mut decision = PolicyDecision::allow();

        for policy in &self.policies {
            if policy.matches(context) {
                let result = policy.evaluate(context);
                decision = decision.merge(result);

                // Short-circuit on deny
                if decision.action == PolicyAction::Deny {
                    return decision;
                }
            }
        }

        decision
    }

    /// Get required QoM profile for an SType
    pub fn required_profile(&self, stype: &SType) -> Option<&str> {
        for policy in &self.policies {
            if let Some(ref qom_rule) = policy.qom_override {
                if policy.matches_stype(stype) {
                    return Some(&qom_rule.profile);
                }
            }
        }
        self.default_profile.as_deref()
    }

    /// Load policies from configuration
    pub fn from_config(config: PolicyConfig) -> Self {
        let mut engine = Self::new();
        engine.default_profile = config.default_profile;
        engine.policies = config.policies;
        engine
    }
}

/// Policy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Policy name
    pub name: String,

    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// SType patterns this policy applies to
    #[serde(default)]
    pub stype_patterns: Vec<StypePattern>,

    /// Operation types this policy applies to
    #[serde(default)]
    pub operations: HashSet<Operation>,

    /// Access control rules
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_control: Option<AccessControlRule>,

    /// QoM profile override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qom_override: Option<QomOverride>,

    /// Rate limiting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimit>,

    /// Custom constraints
    #[serde(default)]
    pub constraints: Vec<Constraint>,

    /// Priority (higher = evaluated first)
    #[serde(default)]
    pub priority: i32,

    /// Whether this policy is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl Policy {
    /// Create a new policy
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            stype_patterns: Vec::new(),
            operations: HashSet::new(),
            access_control: None,
            qom_override: None,
            rate_limit: None,
            constraints: Vec::new(),
            priority: 0,
            enabled: true,
        }
    }

    /// Add an SType pattern
    pub fn with_stype_pattern(mut self, pattern: StypePattern) -> Self {
        self.stype_patterns.push(pattern);
        self
    }

    /// Add operations
    pub fn with_operations(mut self, ops: impl IntoIterator<Item = Operation>) -> Self {
        self.operations.extend(ops);
        self
    }

    /// Set access control
    pub fn with_access_control(mut self, rule: AccessControlRule) -> Self {
        self.access_control = Some(rule);
        self
    }

    /// Set QoM override
    pub fn with_qom_override(mut self, profile: impl Into<String>) -> Self {
        self.qom_override = Some(QomOverride {
            profile: profile.into(),
            reason: None,
        });
        self
    }

    /// Check if policy matches context
    pub fn matches(&self, context: &PolicyContext) -> bool {
        if !self.enabled {
            return false;
        }

        // Check operations
        if !self.operations.is_empty() && !self.operations.contains(&context.operation) {
            return false;
        }

        // Check SType patterns
        if !self.stype_patterns.is_empty() {
            let matches_stype = self.stype_patterns.iter().any(|p| p.matches(&context.stype));
            if !matches_stype {
                return false;
            }
        }

        true
    }

    /// Check if policy matches an SType
    pub fn matches_stype(&self, stype: &SType) -> bool {
        if self.stype_patterns.is_empty() {
            return true;
        }
        self.stype_patterns.iter().any(|p| p.matches(stype))
    }

    /// Evaluate policy against context
    pub fn evaluate(&self, context: &PolicyContext) -> PolicyDecision {
        let mut decision = PolicyDecision::allow();

        // Check access control
        if let Some(ref acl) = self.access_control {
            if !acl.is_allowed(&context.principal, &context.operation) {
                return PolicyDecision::deny(format!(
                    "Access denied: {} not allowed for operation {:?}",
                    context.principal.as_deref().unwrap_or("anonymous"),
                    context.operation
                ));
            }
        }

        // Check constraints
        for constraint in &self.constraints {
            if !constraint.evaluate(context) {
                decision.warnings.push(format!(
                    "Constraint '{}' not satisfied",
                    constraint.name
                ));
                if constraint.required {
                    return PolicyDecision::deny(format!(
                        "Required constraint '{}' not satisfied",
                        constraint.name
                    ));
                }
            }
        }

        // Apply QoM override
        if let Some(ref qom) = self.qom_override {
            decision.required_profile = Some(qom.profile.clone());
        }

        decision
    }
}

/// SType pattern for matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StypePattern {
    /// Namespace pattern (supports wildcards)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,

    /// Domain pattern (supports wildcards)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,

    /// Name pattern (supports wildcards)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Version constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<VersionConstraint>,
}

impl StypePattern {
    /// Create a pattern matching all STypes
    pub fn all() -> Self {
        Self {
            namespace: None,
            domain: None,
            name: None,
            version: None,
        }
    }

    /// Create a pattern matching a namespace
    pub fn namespace(ns: impl Into<String>) -> Self {
        Self {
            namespace: Some(ns.into()),
            domain: None,
            name: None,
            version: None,
        }
    }

    /// Create a pattern matching namespace and domain
    pub fn namespace_domain(ns: impl Into<String>, domain: impl Into<String>) -> Self {
        Self {
            namespace: Some(ns.into()),
            domain: Some(domain.into()),
            name: None,
            version: None,
        }
    }

    /// Check if pattern matches an SType
    pub fn matches(&self, stype: &SType) -> bool {
        // Check namespace
        if let Some(ref ns_pattern) = self.namespace {
            if !glob_match(ns_pattern, &stype.namespace) {
                return false;
            }
        }

        // Check domain
        if let Some(ref domain_pattern) = self.domain {
            if !glob_match(domain_pattern, &stype.domain) {
                return false;
            }
        }

        // Check name
        if let Some(ref name_pattern) = self.name {
            if !glob_match(name_pattern, &stype.name) {
                return false;
            }
        }

        // Check version
        if let Some(ref version_constraint) = self.version {
            if !version_constraint.matches(stype.major_version) {
                return false;
            }
        }

        true
    }
}

/// Version constraint for pattern matching
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum VersionConstraint {
    #[serde(rename = "eq")]
    Eq { version: u32 },
    #[serde(rename = "gte")]
    Gte { version: u32 },
    #[serde(rename = "lte")]
    Lte { version: u32 },
    #[serde(rename = "range")]
    Range { min: u32, max: u32 },
}

impl VersionConstraint {
    pub fn matches(&self, version: u32) -> bool {
        match self {
            VersionConstraint::Eq { version: v } => version == *v,
            VersionConstraint::Gte { version: v } => version >= *v,
            VersionConstraint::Lte { version: v } => version <= *v,
            VersionConstraint::Range { min, max } => version >= *min && version <= *max,
        }
    }
}

/// Operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    /// Read/fetch schema or data
    Read,
    /// Create new payload
    Create,
    /// Update existing payload
    Update,
    /// Delete payload
    Delete,
    /// Validate payload
    Validate,
    /// Execute tool call
    Execute,
    /// Subscribe to events
    Subscribe,
}

/// Access control rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlRule {
    /// Allowed principals (user/service IDs)
    #[serde(default)]
    pub allow: HashSet<String>,

    /// Denied principals
    #[serde(default)]
    pub deny: HashSet<String>,

    /// Allowed operations per principal
    #[serde(default)]
    pub operation_map: HashMap<String, HashSet<Operation>>,

    /// Default action when no match
    #[serde(default)]
    pub default: AccessDefault,
}

impl AccessControlRule {
    /// Check if principal is allowed for operation
    pub fn is_allowed(&self, principal: &Option<String>, operation: &Operation) -> bool {
        let principal = principal.as_deref().unwrap_or("anonymous");

        // Check explicit deny
        if self.deny.contains(principal) || self.deny.contains("*") {
            return false;
        }

        // Check explicit allow
        if self.allow.contains(principal) || self.allow.contains("*") {
            // Check operation restrictions
            if let Some(ops) = self.operation_map.get(principal) {
                return ops.contains(operation);
            }
            return true;
        }

        // Default action
        matches!(self.default, AccessDefault::Allow)
    }
}

/// Default access action
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessDefault {
    Allow,
    #[default]
    Deny,
}

/// QoM profile override
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QomOverride {
    /// Required profile name
    pub profile: String,

    /// Reason for override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    /// Requests per window
    pub requests: u32,

    /// Window duration in seconds
    pub window_seconds: u32,

    /// Per-principal or global
    #[serde(default)]
    pub per_principal: bool,
}

/// Custom constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    /// Constraint name
    pub name: String,

    /// Constraint expression (simplified)
    pub expression: ConstraintExpr,

    /// Whether constraint is required (deny) or advisory (warn)
    #[serde(default)]
    pub required: bool,
}

impl Constraint {
    /// Evaluate constraint against context
    pub fn evaluate(&self, context: &PolicyContext) -> bool {
        self.expression.evaluate(context)
    }
}

/// Constraint expression
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ConstraintExpr {
    /// Check if metadata contains key
    #[serde(rename = "has_metadata")]
    HasMetadata { key: String },

    /// Check metadata value
    #[serde(rename = "metadata_equals")]
    MetadataEquals { key: String, value: String },

    /// Check payload size
    #[serde(rename = "max_payload_size")]
    MaxPayloadSize { bytes: usize },

    /// Always pass
    #[serde(rename = "always")]
    Always,

    /// Always fail
    #[serde(rename = "never")]
    Never,
}

impl ConstraintExpr {
    pub fn evaluate(&self, context: &PolicyContext) -> bool {
        match self {
            ConstraintExpr::HasMetadata { key } => {
                context.metadata.contains_key(key)
            }
            ConstraintExpr::MetadataEquals { key, value } => {
                context.metadata.get(key).map(|v| v == value).unwrap_or(false)
            }
            ConstraintExpr::MaxPayloadSize { bytes } => {
                context.payload_size.map(|s| s <= *bytes).unwrap_or(true)
            }
            ConstraintExpr::Always => true,
            ConstraintExpr::Never => false,
        }
    }
}

/// Context for policy evaluation
#[derive(Debug, Clone)]
pub struct PolicyContext {
    /// SType being operated on
    pub stype: SType,

    /// Operation type
    pub operation: Operation,

    /// Principal (user/service ID)
    pub principal: Option<String>,

    /// Additional metadata
    pub metadata: HashMap<String, String>,

    /// Payload size (if applicable)
    pub payload_size: Option<usize>,
}

impl PolicyContext {
    /// Create a new policy context
    pub fn new(stype: SType, operation: Operation) -> Self {
        Self {
            stype,
            operation,
            principal: None,
            metadata: HashMap::new(),
            payload_size: None,
        }
    }

    /// Set principal
    pub fn with_principal(mut self, principal: impl Into<String>) -> Self {
        self.principal = Some(principal.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Set payload size
    pub fn with_payload_size(mut self, size: usize) -> Self {
        self.payload_size = Some(size);
        self
    }
}

/// Policy decision result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    /// Action to take
    pub action: PolicyAction,

    /// Reason for decision
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Required QoM profile (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_profile: Option<String>,

    /// Warnings (non-blocking)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

impl PolicyDecision {
    /// Create an allow decision
    pub fn allow() -> Self {
        Self {
            action: PolicyAction::Allow,
            reason: None,
            required_profile: None,
            warnings: Vec::new(),
        }
    }

    /// Create a deny decision
    pub fn deny(reason: impl Into<String>) -> Self {
        Self {
            action: PolicyAction::Deny,
            reason: Some(reason.into()),
            required_profile: None,
            warnings: Vec::new(),
        }
    }

    /// Merge with another decision (more restrictive wins)
    pub fn merge(mut self, other: PolicyDecision) -> Self {
        // Deny takes precedence
        if other.action == PolicyAction::Deny {
            return other;
        }

        // Merge warnings
        self.warnings.extend(other.warnings);

        // Use more specific profile
        if other.required_profile.is_some() {
            self.required_profile = other.required_profile;
        }

        self
    }

    /// Check if allowed
    pub fn is_allowed(&self) -> bool {
        self.action == PolicyAction::Allow
    }
}

/// Policy action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyAction {
    Allow,
    Deny,
}

/// Policy configuration for loading from file
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicyConfig {
    /// Default QoM profile
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_profile: Option<String>,

    /// Policies
    #[serde(default)]
    pub policies: Vec<Policy>,
}

// Simple glob matching (supports * wildcard)
fn glob_match(pattern: &str, text: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    if !pattern.contains('*') {
        return pattern == text;
    }

    let parts: Vec<&str> = pattern.split('*').collect();
    let mut pos = 0;

    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }

        match text[pos..].find(part) {
            Some(idx) => {
                // First part must match at start
                if i == 0 && idx != 0 {
                    return false;
                }
                pos += idx + part.len();
            }
            None => return false,
        }
    }

    // Last part must match at end
    if let Some(last) = parts.last() {
        if !last.is_empty() && !text.ends_with(last) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_stype() -> SType {
        SType {
            namespace: "eval".to_string(),
            domain: "rag".to_string(),
            name: "RAGQuery".to_string(),
            major_version: 1,
        }
    }

    #[test]
    fn test_pattern_all() {
        let pattern = StypePattern::all();
        assert!(pattern.matches(&test_stype()));
    }

    #[test]
    fn test_pattern_namespace() {
        let pattern = StypePattern::namespace("eval");
        assert!(pattern.matches(&test_stype()));

        let pattern = StypePattern::namespace("org");
        assert!(!pattern.matches(&test_stype()));
    }

    #[test]
    fn test_pattern_wildcard() {
        let pattern = StypePattern {
            namespace: Some("ev*".to_string()),
            domain: None,
            name: None,
            version: None,
        };
        assert!(pattern.matches(&test_stype()));
    }

    #[test]
    fn test_policy_engine() {
        let mut engine = PolicyEngine::new();

        // Add policy for eval namespace requiring strict QoM
        let policy = Policy::new("eval-strict")
            .with_stype_pattern(StypePattern::namespace("eval"))
            .with_qom_override("qom-strict-argcheck");

        engine.add_policy(policy);

        let context = PolicyContext::new(test_stype(), Operation::Execute);
        let decision = engine.evaluate(&context);

        assert!(decision.is_allowed());
        assert_eq!(decision.required_profile, Some("qom-strict-argcheck".to_string()));
    }

    #[test]
    fn test_access_control_deny() {
        let mut engine = PolicyEngine::new();

        let policy = Policy::new("restricted")
            .with_stype_pattern(StypePattern::namespace("eval"))
            .with_access_control(AccessControlRule {
                allow: HashSet::from(["admin".to_string()]),
                deny: HashSet::new(),
                operation_map: HashMap::new(),
                default: AccessDefault::Deny,
            });

        engine.add_policy(policy);

        // Anonymous user should be denied
        let context = PolicyContext::new(test_stype(), Operation::Execute);
        let decision = engine.evaluate(&context);
        assert!(!decision.is_allowed());

        // Admin should be allowed
        let context = PolicyContext::new(test_stype(), Operation::Execute)
            .with_principal("admin");
        let decision = engine.evaluate(&context);
        assert!(decision.is_allowed());
    }

    #[test]
    fn test_glob_matching() {
        assert!(glob_match("*", "anything"));
        assert!(glob_match("eval", "eval"));
        assert!(!glob_match("eval", "org"));
        assert!(glob_match("ev*", "eval"));
        assert!(glob_match("*val", "eval"));
        assert!(glob_match("e*l", "eval"));
        assert!(glob_match("*a*", "eval"));
    }
}
