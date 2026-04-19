//! Core proxy logic

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use mpl_core::envelope::MplEnvelope;
use mpl_core::hash::verify_hash;
use mpl_core::qom::{QomMetrics, QomProfile};
use mpl_core::validation::SchemaValidator;

use crate::config::{ProxyConfig, ProxyMode};
use crate::metrics::MetricsState;

/// MPL headers
pub const HEADER_STYPE: &str = "X-MPL-SType";
pub const HEADER_PROFILE: &str = "X-MPL-Profile";
pub const HEADER_SEM_HASH: &str = "X-MPL-Sem-Hash";
pub const HEADER_QOM_RESULT: &str = "X-MPL-QoM-Result";

/// Validation result for a request
#[derive(Debug, Clone, Serialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub stype: Option<String>,
    pub schema_valid: bool,
    pub qom_passed: bool,
    pub hash_valid: bool,
    pub errors: Vec<String>,
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self {
            valid: true,
            stype: None,
            schema_valid: true,
            qom_passed: true,
            hash_valid: true,
            errors: Vec::new(),
        }
    }
}

/// Shared proxy state
pub struct ProxyState {
    /// Configuration
    pub config: ProxyConfig,

    /// HTTP client for upstream requests
    pub client: Client,

    /// Schema validator
    pub validator: SchemaValidator,

    /// QoM profiles
    pub profiles: Vec<QomProfile>,

    /// Metrics
    pub metrics: Arc<MetricsState>,
}

impl ProxyState {
    /// Create a new proxy state from configuration
    pub async fn new(config: ProxyConfig) -> Result<Self> {
        // Build HTTP client with configured timeouts
        let client = Client::builder()
            .connect_timeout(config.transport.connect_timeout())
            .timeout(config.transport.request_timeout())
            .pool_idle_timeout(config.transport.idle_timeout())
            .build()?;

        let mut validator = SchemaValidator::new();

        // Load schemas from registry if it's a local path
        let registry_path = &config.mpl.registry;
        if Path::new(registry_path).exists() {
            Self::load_schemas_from_registry(&mut validator, registry_path)?;
        }

        let profiles = vec![QomProfile::basic(), QomProfile::strict_argcheck()];

        let metrics = Arc::new(MetricsState::new());

        info!("Proxy state initialized");
        info!("Mode: {:?}", config.mpl.mode);
        info!("Loaded {} schemas", validator.registered_stypes().len());
        info!("Loaded {} QoM profiles", profiles.len());

        Ok(Self {
            config,
            client,
            validator,
            profiles,
            metrics,
        })
    }

    /// Load schemas from local registry directory
    fn load_schemas_from_registry(validator: &mut SchemaValidator, registry_path: &str) -> Result<()> {
        let stypes_path = Path::new(registry_path).join("stypes");
        if !stypes_path.exists() {
            debug!("Registry stypes path does not exist: {}", stypes_path.display());
            return Ok(());
        }

        // Walk the stypes directory structure: namespace/domain/Name/vN/schema.json
        Self::walk_registry_dir(validator, &stypes_path, Vec::new())?;
        Ok(())
    }

    fn walk_registry_dir(
        validator: &mut SchemaValidator,
        path: &Path,
        parts: Vec<String>,
    ) -> Result<()> {
        if !path.is_dir() {
            return Ok(());
        }

        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if entry_path.is_dir() {
                let mut new_parts = parts.clone();
                new_parts.push(name);
                Self::walk_registry_dir(validator, &entry_path, new_parts)?;
            } else if name == "schema.json" && parts.len() >= 4 {
                // We have: namespace/domain/Name/vN/schema.json
                // parts = [namespace, domain, Name, vN]
                let version_str = &parts[parts.len() - 1];
                if let Some(version) = version_str.strip_prefix('v') {
                    if version.parse::<u32>().is_ok() {
                        let namespace = parts[..parts.len() - 2].join(".");
                        let name = &parts[parts.len() - 2];
                        let stype = format!(
                            "{}.{}.{}",
                            namespace,
                            name,
                            version_str
                        );

                        // Read and register schema
                        if let Ok(schema_content) = std::fs::read_to_string(&entry_path) {
                            if validator.register_json(&stype, &schema_content).is_ok() {
                                debug!("Registered schema for {}", stype);
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Validate an MPL request
    pub fn validate_request(&self, envelope: &MplEnvelope) -> ValidationResult {
        let mut result = ValidationResult {
            stype: Some(envelope.stype.clone()),
            ..Default::default()
        };

        // Schema validation
        if self.config.mpl.enforce_schema {
            match self.validator.validate(&envelope.stype, &envelope.payload) {
                Ok(validation) => {
                    result.schema_valid = validation.valid;
                    if !validation.valid {
                        result.valid = false;
                        for err in validation.errors {
                            result.errors.push(format!("Schema error at {}: {}", err.path, err.message));
                        }
                    }
                }
                Err(e) => {
                    // Unknown SType - check mode
                    if self.is_strict() {
                        result.valid = false;
                        result.schema_valid = false;
                        result.errors.push(format!("Unknown SType: {} ({})", envelope.stype, e));
                    } else {
                        warn!("Unknown SType: {}, allowing in transparent mode", envelope.stype);
                    }
                }
            }
        }

        // Hash verification (if provided)
        if let Some(ref expected_hash) = envelope.sem_hash {
            match verify_hash(&envelope.payload, expected_hash) {
                Ok(valid) => {
                    result.hash_valid = valid;
                    if !valid {
                        result.valid = false;
                        result.errors.push("Semantic hash mismatch".to_string());
                    }
                }
                Err(e) => {
                    result.hash_valid = false;
                    result.valid = false;
                    result.errors.push(format!("Hash verification failed: {}", e));
                }
            }
        }

        // QoM evaluation
        if let Some(profile) = self.active_profile() {
            let metrics = if result.schema_valid {
                QomMetrics::schema_valid()
            } else {
                QomMetrics::schema_invalid()
            };

            let evaluation = profile.evaluate(&metrics);
            result.qom_passed = evaluation.meets_profile;

            if !evaluation.meets_profile {
                result.valid = false;
                for failure in evaluation.failures {
                    result
                        .errors
                        .push(format!("QoM breach: {} < {}", failure.metric, failure.threshold));
                }
            }
        }

        // Update metrics
        self.metrics.inc_requests();
        if result.schema_valid {
            self.metrics.inc_schema_pass();
        } else {
            self.metrics.inc_schema_fail();
        }
        if result.qom_passed {
            self.metrics.inc_qom_pass();
        } else {
            self.metrics.inc_qom_fail();
        }

        result
    }

    /// Forward a request to the upstream server
    pub async fn forward_request(
        &self,
        path: String,
        request: Request<Body>,
    ) -> Result<Response<Body>> {
        let upstream = &self.config.transport.upstream;
        let uri = format!("http://{}/{}", upstream, path);

        debug!("Forwarding to: {}", uri);

        // Extract headers
        let method = request.method().clone();
        let headers = request.headers().clone();
        let stype_header = headers
            .get(HEADER_STYPE)
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        // Read body
        let body_bytes = axum::body::to_bytes(request.into_body(), usize::MAX).await?;

        // Try to parse as MPL envelope or create one from headers
        let envelope = if let Ok(env) = serde_json::from_slice::<MplEnvelope>(&body_bytes) {
            Some(env)
        } else if let Some(stype) = stype_header {
            // Create envelope from headers + body
            if let Ok(payload) = serde_json::from_slice(&body_bytes) {
                Some(MplEnvelope::new(stype, payload))
            } else {
                None
            }
        } else {
            None
        };

        // Validate if we have an envelope
        let validation_result = if let Some(ref env) = envelope {
            let result = self.validate_request(env);

            // In strict mode, block invalid requests
            if !result.valid && self.is_strict() {
                let error_response = serde_json::json!({
                    "error": "E-SCHEMA-FIDELITY",
                    "message": "Request validation failed",
                    "details": result.errors,
                });

                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("content-type", "application/json")
                    .header(HEADER_QOM_RESULT, if result.qom_passed { "pass" } else { "fail" })
                    .body(Body::from(serde_json::to_string(&error_response)?))?);
            }

            Some(result)
        } else {
            None
        };

        // Build upstream request
        let mut req_builder = self.client.request(method, &uri);

        for (name, value) in headers.iter() {
            if name != "host" {
                req_builder = req_builder.header(name, value);
            }
        }

        let upstream_response = req_builder.body(body_bytes.to_vec()).send().await?;

        // Convert response back to axum
        let status = upstream_response.status();
        let response_headers = upstream_response.headers().clone();
        let body = upstream_response.bytes().await?;

        let mut response = Response::builder().status(status);

        for (name, value) in response_headers.iter() {
            response = response.header(name, value);
        }

        // Add MPL headers to response
        if let Some(ref result) = validation_result {
            response = response.header(HEADER_QOM_RESULT, if result.qom_passed { "pass" } else { "fail" });
        }

        Ok(response.body(Body::from(body))?)
    }

    /// Get the active QoM profile
    pub fn active_profile(&self) -> Option<&QomProfile> {
        let profile_name = self.config.mpl.required_profile.as_ref()?;
        self.profiles.iter().find(|p| &p.name == profile_name)
    }

    /// Check if we're in strict mode
    pub fn is_strict(&self) -> bool {
        matches!(self.config.mpl.mode, ProxyMode::Strict)
    }
}

/// AI-ALPN Client Hello message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiAlpnClientHello {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub version: String,
    pub stypes: Vec<String>,
    #[serde(default)]
    pub qom_profiles: Vec<String>,
}

/// AI-ALPN Server Select message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiAlpnServerSelect {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub common_stypes: Vec<String>,
    pub selected_profile: Option<String>,
    #[serde(default)]
    pub extensions: serde_json::Value,
}

impl ProxyState {
    /// Handle AI-ALPN handshake
    pub fn handle_handshake(&self, hello: AiAlpnClientHello) -> AiAlpnServerSelect {
        // Find common STypes
        let server_stypes: Vec<String> = self.validator.registered_stypes().iter().map(|s| s.to_string()).collect();
        let common_stypes: Vec<String> = hello
            .stypes
            .iter()
            .filter(|s| server_stypes.contains(s))
            .cloned()
            .collect();

        // Select profile
        let selected_profile = hello
            .qom_profiles
            .iter()
            .find(|p| self.profiles.iter().any(|sp| &sp.name == *p))
            .cloned()
            .or_else(|| self.config.mpl.required_profile.clone());

        self.metrics.inc_handshakes();

        AiAlpnServerSelect {
            msg_type: "ai-alpn-select".to_string(),
            common_stypes,
            selected_profile,
            extensions: serde_json::json!({}),
        }
    }
}
