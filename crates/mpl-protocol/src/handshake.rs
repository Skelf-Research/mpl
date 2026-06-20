//! AI-ALPN Handshake
//!
//! Capability negotiation protocol modeled after TLS ALPN.
//! Before exchanging work, peers negotiate protocols, models, STypes,
//! tools, QoM profiles, and policies.

use serde::{Deserialize, Serialize};

use crate::MPL_VERSION;

/// Client's initial handshake message proposing capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientHello {
    /// MPL protocol version
    pub mpl_version: String,

    /// Supported MCP/A2A protocol versions
    #[serde(default)]
    pub protocols: Vec<String>,

    /// Supported/required STypes
    #[serde(default)]
    pub stypes: Vec<String>,

    /// Requested tools
    #[serde(default)]
    pub tools: Vec<ToolRequest>,

    /// Requested QoM profile
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,

    /// Policy references
    #[serde(default)]
    pub policies: Vec<String>,

    /// Optional feature flags
    #[serde(default)]
    pub features: Vec<String>,

    /// Model preferences (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Client identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
}

impl ClientHello {
    /// Create a new ClientHello with default MPL version
    pub fn new() -> Self {
        Self {
            mpl_version: MPL_VERSION.to_string(),
            protocols: Vec::new(),
            stypes: Vec::new(),
            tools: Vec::new(),
            profile: None,
            policies: Vec::new(),
            features: Vec::new(),
            model: None,
            client_id: None,
        }
    }

    /// Add supported protocols
    pub fn with_protocols(mut self, protocols: Vec<String>) -> Self {
        self.protocols = protocols;
        self
    }

    /// Add required STypes
    pub fn with_stypes(mut self, stypes: Vec<String>) -> Self {
        self.stypes = stypes;
        self
    }

    /// Add tool requests
    pub fn with_tools(mut self, tools: Vec<ToolRequest>) -> Self {
        self.tools = tools;
        self
    }

    /// Set QoM profile
    pub fn with_profile(mut self, profile: impl Into<String>) -> Self {
        self.profile = Some(profile.into());
        self
    }

    /// Add policy references
    pub fn with_policies(mut self, policies: Vec<String>) -> Self {
        self.policies = policies;
        self
    }

    /// Add feature flags
    pub fn with_features(mut self, features: Vec<String>) -> Self {
        self.features = features;
        self
    }
}

impl Default for ClientHello {
    fn default() -> Self {
        Self::new()
    }
}

/// Tool request in handshake
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRequest {
    /// Tool identifier
    pub id: String,

    /// Required features for this tool
    #[serde(default)]
    pub features: Vec<String>,
}

impl ToolRequest {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            features: Vec::new(),
        }
    }

    pub fn with_features(mut self, features: Vec<String>) -> Self {
        self.features = features;
        self
    }
}

/// Server's response selecting compatible capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSelect {
    /// MPL protocol version
    pub mpl_version: String,

    /// Selected protocol
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,

    /// Supported STypes from client's list
    #[serde(default)]
    pub stypes: Vec<String>,

    /// Available tools from client's list
    #[serde(default)]
    pub tools: Vec<ToolResponse>,

    /// Selected QoM profile (may differ from requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,

    /// Accepted policies
    #[serde(default)]
    pub policies: Vec<String>,

    /// Supported features
    #[serde(default)]
    pub features: Vec<String>,

    /// Downgrade explanations
    #[serde(default)]
    pub downgrades: Vec<Downgrade>,

    /// Whether negotiation succeeded
    pub success: bool,

    /// Error message if negotiation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Server identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
}

impl ServerSelect {
    /// Create a successful response
    pub fn success() -> Self {
        Self {
            mpl_version: MPL_VERSION.to_string(),
            protocol: None,
            stypes: Vec::new(),
            tools: Vec::new(),
            profile: None,
            policies: Vec::new(),
            features: Vec::new(),
            downgrades: Vec::new(),
            success: true,
            error: None,
            server_id: None,
        }
    }

    /// Create a failed response
    pub fn failed(error: impl Into<String>) -> Self {
        Self {
            mpl_version: MPL_VERSION.to_string(),
            protocol: None,
            stypes: Vec::new(),
            tools: Vec::new(),
            profile: None,
            policies: Vec::new(),
            features: Vec::new(),
            downgrades: Vec::new(),
            success: false,
            error: Some(error.into()),
            server_id: None,
        }
    }

    /// Set selected protocol
    pub fn with_protocol(mut self, protocol: impl Into<String>) -> Self {
        self.protocol = Some(protocol.into());
        self
    }

    /// Set supported STypes
    pub fn with_stypes(mut self, stypes: Vec<String>) -> Self {
        self.stypes = stypes;
        self
    }

    /// Set available tools
    pub fn with_tools(mut self, tools: Vec<ToolResponse>) -> Self {
        self.tools = tools;
        self
    }

    /// Set selected profile
    pub fn with_profile(mut self, profile: impl Into<String>) -> Self {
        self.profile = Some(profile.into());
        self
    }

    /// Add a downgrade
    pub fn with_downgrade(mut self, downgrade: Downgrade) -> Self {
        self.downgrades.push(downgrade);
        self
    }
}

/// Tool availability response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResponse {
    /// Tool identifier
    pub id: String,

    /// Whether tool is available
    pub available: bool,

    /// Supported features for this tool
    #[serde(default)]
    pub features: Vec<String>,

    /// Reason if not available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl ToolResponse {
    pub fn available(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            available: true,
            features: Vec::new(),
            reason: None,
        }
    }

    pub fn unavailable(id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            available: false,
            features: Vec::new(),
            reason: Some(reason.into()),
        }
    }

    pub fn with_features(mut self, features: Vec<String>) -> Self {
        self.features = features;
        self
    }
}

/// Downgrade explanation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Downgrade {
    /// What was downgraded (stype, tool, profile, feature)
    pub category: DowngradeCategory,

    /// Original requested value
    pub requested: String,

    /// What was selected instead (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected: Option<String>,

    /// Reason for downgrade
    pub reason: String,
}

impl Downgrade {
    pub fn new(
        category: DowngradeCategory,
        requested: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            category,
            requested: requested.into(),
            selected: None,
            reason: reason.into(),
        }
    }

    pub fn with_selected(mut self, selected: impl Into<String>) -> Self {
        self.selected = Some(selected.into());
        self
    }
}

/// Category of downgrade
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DowngradeCategory {
    Protocol,
    Stype,
    Tool,
    Profile,
    Policy,
    Feature,
    Model,
}

/// Negotiate capabilities between client and server
pub fn negotiate(client: &ClientHello, server_capabilities: &ServerCapabilities) -> ServerSelect {
    let mut response = ServerSelect::success();
    let mut downgrades = Vec::new();

    // Check protocol compatibility
    if let Some(protocol) = client
        .protocols
        .iter()
        .find(|p| server_capabilities.protocols.contains(p))
    {
        response.protocol = Some(protocol.clone());
    } else if !client.protocols.is_empty() {
        downgrades.push(Downgrade::new(
            DowngradeCategory::Protocol,
            client.protocols.join(", "),
            "No compatible protocol found",
        ));
    }

    // Check STypes
    for stype in &client.stypes {
        if server_capabilities.stypes.contains(stype) {
            response.stypes.push(stype.clone());
        } else {
            downgrades.push(Downgrade::new(
                DowngradeCategory::Stype,
                stype,
                "SType not supported",
            ));
        }
    }

    // Check tools
    for tool_req in &client.tools {
        if let Some(server_tool) = server_capabilities
            .tools
            .iter()
            .find(|t| t.id == tool_req.id)
        {
            let supported_features: Vec<_> = tool_req
                .features
                .iter()
                .filter(|f| server_tool.features.contains(f))
                .cloned()
                .collect();

            let unsupported: Vec<_> = tool_req
                .features
                .iter()
                .filter(|f| !server_tool.features.contains(f))
                .cloned()
                .collect();

            response
                .tools
                .push(ToolResponse::available(&tool_req.id).with_features(supported_features));

            for feature in unsupported {
                downgrades.push(Downgrade::new(
                    DowngradeCategory::Feature,
                    format!("{}:{}", tool_req.id, feature),
                    "Feature not supported for tool",
                ));
            }
        } else {
            response.tools.push(ToolResponse::unavailable(
                &tool_req.id,
                "Tool not available",
            ));
            downgrades.push(Downgrade::new(
                DowngradeCategory::Tool,
                &tool_req.id,
                "Tool not available",
            ));
        }
    }

    // Check profile
    if let Some(requested_profile) = &client.profile {
        if server_capabilities.profiles.contains(requested_profile) {
            response.profile = Some(requested_profile.clone());
        } else if let Some(fallback) = server_capabilities.profiles.first() {
            response.profile = Some(fallback.clone());
            downgrades.push(
                Downgrade::new(
                    DowngradeCategory::Profile,
                    requested_profile,
                    "Requested profile not available",
                )
                .with_selected(fallback),
            );
        }
    }

    // Check policies
    for policy in &client.policies {
        if server_capabilities.policies.contains(policy) {
            response.policies.push(policy.clone());
        } else {
            downgrades.push(Downgrade::new(
                DowngradeCategory::Policy,
                policy,
                "Policy not supported",
            ));
        }
    }

    response.downgrades = downgrades;
    response
}

/// Server's available capabilities for negotiation
#[derive(Debug, Clone, Default)]
pub struct ServerCapabilities {
    pub protocols: Vec<String>,
    pub stypes: Vec<String>,
    pub tools: Vec<ToolCapability>,
    pub profiles: Vec<String>,
    pub policies: Vec<String>,
    pub features: Vec<String>,
}

/// Tool capability declaration
#[derive(Debug, Clone)]
pub struct ToolCapability {
    pub id: String,
    pub features: Vec<String>,
}

impl ToolCapability {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            features: Vec::new(),
        }
    }

    pub fn with_features(mut self, features: Vec<String>) -> Self {
        self.features = features;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_hello_builder() {
        let hello = ClientHello::new()
            .with_stypes(vec!["org.calendar.Event.v1".to_string()])
            .with_tools(vec![ToolRequest::new("calendar.create.v1")])
            .with_profile("qom-basic");

        assert_eq!(hello.stypes.len(), 1);
        assert_eq!(hello.tools.len(), 1);
        assert_eq!(hello.profile, Some("qom-basic".to_string()));
    }

    #[test]
    fn test_negotiation_success() {
        let client = ClientHello::new()
            .with_stypes(vec!["org.calendar.Event.v1".to_string()])
            .with_profile("qom-basic");

        let server = ServerCapabilities {
            stypes: vec!["org.calendar.Event.v1".to_string()],
            profiles: vec!["qom-basic".to_string()],
            ..Default::default()
        };

        let response = negotiate(&client, &server);
        assert!(response.success);
        assert!(response
            .stypes
            .contains(&"org.calendar.Event.v1".to_string()));
        assert_eq!(response.profile, Some("qom-basic".to_string()));
    }

    #[test]
    fn test_negotiation_downgrade() {
        let client = ClientHello::new().with_profile("qom-strict-argcheck");

        let server = ServerCapabilities {
            profiles: vec!["qom-basic".to_string()],
            ..Default::default()
        };

        let response = negotiate(&client, &server);
        assert!(response.success);
        assert_eq!(response.profile, Some("qom-basic".to_string()));
        assert!(!response.downgrades.is_empty());
        assert_eq!(response.downgrades[0].category, DowngradeCategory::Profile);
    }
}
