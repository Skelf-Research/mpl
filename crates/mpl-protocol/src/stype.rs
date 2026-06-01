//! Semantic Types (STypes)
//!
//! STypes are globally unique, versioned identifiers that declare the intent
//! and schema of a payload. Format: `namespace.domain.Intent.vMajor`

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::error::{MplError, Result};

/// A Semantic Type identifier
///
/// Format: `namespace.domain.Name.vMajor`
/// Example: `org.calendar.Event.v1`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SType {
    /// Organizational namespace (e.g., "org", "com.acme")
    pub namespace: String,
    /// Domain within namespace (e.g., "calendar", "finance")
    pub domain: String,
    /// Intent/entity name (e.g., "Event", "InvestmentRecommendation")
    pub name: String,
    /// Major version number
    pub major_version: u32,
}

impl SType {
    /// Create a new SType
    pub fn new(namespace: &str, domain: &str, name: &str, major_version: u32) -> Self {
        Self {
            namespace: namespace.to_string(),
            domain: domain.to_string(),
            name: name.to_string(),
            major_version,
        }
    }

    /// Maximum length for SType string input
    pub const MAX_STYPE_LENGTH: usize = 256;
    /// Maximum depth for namespace (number of dot-separated parts)
    pub const MAX_NAMESPACE_DEPTH: usize = 10;
    /// Maximum version number
    pub const MAX_VERSION: u32 = 999;

    /// Parse an SType from a string
    ///
    /// Accepts formats:
    /// - `namespace.domain.Name.vN` (e.g., `org.calendar.Event.v1`)
    /// - `urn:stype:namespace.domain.Name.vN` (full URN format)
    pub fn parse(s: &str) -> Result<Self> {
        // Input length validation
        if s.is_empty() {
            return Err(MplError::InvalidSType {
                stype: s.to_string(),
                reason: "SType cannot be empty".to_string(),
            });
        }
        if s.len() > Self::MAX_STYPE_LENGTH {
            return Err(MplError::InvalidSType {
                stype: format!("{}...", &s[..50]),
                reason: format!("SType exceeds maximum length of {} characters", Self::MAX_STYPE_LENGTH),
            });
        }

        let s = s.strip_prefix("urn:stype:").unwrap_or(s);

        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() < 4 {
            return Err(MplError::InvalidSType {
                stype: s.to_string(),
                reason: "Expected format: namespace.domain.Name.vN".to_string(),
            });
        }
        if parts.len() > Self::MAX_NAMESPACE_DEPTH + 3 {
            return Err(MplError::InvalidSType {
                stype: s.to_string(),
                reason: format!("SType has too many parts (max {} namespace depth)", Self::MAX_NAMESPACE_DEPTH),
            });
        }

        // Last part should be version (vN)
        // Safety: parts.len() >= 4 checked above, so last() is guaranteed Some
        let version_part = parts.last().ok_or_else(|| MplError::InvalidSType {
            stype: s.to_string(),
            reason: "Internal error: missing version part".to_string(),
        })?;
        if !version_part.starts_with('v') {
            return Err(MplError::InvalidSType {
                stype: s.to_string(),
                reason: format!("Version must start with 'v', got: {}", version_part),
            });
        }

        let major_version: u32 = version_part[1..].parse().map_err(|_| MplError::InvalidSType {
            stype: s.to_string(),
            reason: format!("Invalid version number: {}", version_part),
        })?;

        // Version bounds check
        if major_version > Self::MAX_VERSION {
            return Err(MplError::InvalidSType {
                stype: s.to_string(),
                reason: format!("Version {} exceeds maximum of {}", major_version, Self::MAX_VERSION),
            });
        }

        // Second to last is the name
        let name = parts[parts.len() - 2].to_string();

        // Everything before that is namespace.domain
        // For simplicity, first part is namespace, second is domain
        // This handles: org.calendar.Event.v1 -> namespace=org, domain=calendar
        // And: com.acme.finance.Trade.v1 -> namespace=com.acme, domain=finance
        let namespace_domain_parts = &parts[..parts.len() - 2];
        if namespace_domain_parts.len() < 2 {
            return Err(MplError::InvalidSType {
                stype: s.to_string(),
                reason: "Need at least namespace and domain".to_string(),
            });
        }

        // Safety: namespace_domain_parts.len() >= 2 checked above, so last() is guaranteed Some
        let domain = namespace_domain_parts.last().ok_or_else(|| MplError::InvalidSType {
            stype: s.to_string(),
            reason: "Internal error: missing domain part".to_string(),
        })?.to_string();
        let namespace = namespace_domain_parts[..namespace_domain_parts.len() - 1].join(".");

        Ok(Self {
            namespace,
            domain,
            name,
            major_version,
        })
    }

    /// Get the short identifier (without URN prefix)
    pub fn id(&self) -> String {
        format!(
            "{}.{}.{}.v{}",
            self.namespace, self.domain, self.name, self.major_version
        )
    }

    /// Get the full URN
    pub fn urn(&self) -> String {
        format!("urn:stype:{}", self.id())
    }

    /// Get the registry path for this SType's schema
    pub fn registry_path(&self) -> String {
        format!(
            "/stypes/{}/{}/{}/v{}/schema.json",
            self.namespace.replace('.', "/"),
            self.domain,
            self.name,
            self.major_version
        )
    }
}

impl fmt::Display for SType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id())
    }
}

impl FromStr for SType {
    type Err = MplError;

    fn from_str(s: &str) -> Result<Self> {
        Self::parse(s)
    }
}

impl TryFrom<String> for SType {
    type Error = MplError;

    fn try_from(s: String) -> Result<Self> {
        Self::parse(&s)
    }
}

impl From<SType> for String {
    fn from(stype: SType) -> Self {
        stype.id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let stype = SType::parse("org.calendar.Event.v1").unwrap();
        assert_eq!(stype.namespace, "org");
        assert_eq!(stype.domain, "calendar");
        assert_eq!(stype.name, "Event");
        assert_eq!(stype.major_version, 1);
    }

    #[test]
    fn test_parse_nested_namespace() {
        let stype = SType::parse("com.acme.finance.Trade.v2").unwrap();
        assert_eq!(stype.namespace, "com.acme");
        assert_eq!(stype.domain, "finance");
        assert_eq!(stype.name, "Trade");
        assert_eq!(stype.major_version, 2);
    }

    #[test]
    fn test_parse_urn() {
        let stype = SType::parse("urn:stype:org.calendar.Event.v1").unwrap();
        assert_eq!(stype.id(), "org.calendar.Event.v1");
    }

    #[test]
    fn test_registry_path() {
        let stype = SType::parse("org.calendar.Event.v1").unwrap();
        assert_eq!(
            stype.registry_path(),
            "/stypes/org/calendar/Event/v1/schema.json"
        );
    }

    #[test]
    fn test_roundtrip() {
        let original = "org.finance.InvestmentRecommendation.v1";
        let stype = SType::parse(original).unwrap();
        assert_eq!(stype.id(), original);
    }
}
