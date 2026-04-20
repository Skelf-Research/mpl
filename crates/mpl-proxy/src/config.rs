//! Proxy configuration
//!
//! Configuration can be loaded from:
//! 1. YAML file (default: mpl-config.yaml)
//! 2. Environment variables (MPL_* prefix)
//! 3. CLI arguments (highest priority)

use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;

/// Main proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub transport: TransportConfig,
    pub mpl: MplConfig,
    pub observability: ObservabilityConfig,
    #[serde(default)]
    pub routing: Vec<RouteConfig>,
    #[serde(default)]
    pub limits: ResourceLimits,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            transport: TransportConfig::default(),
            mpl: MplConfig::default(),
            observability: ObservabilityConfig::default(),
            routing: Vec::new(),
            limits: ResourceLimits::default(),
        }
    }
}

impl ProxyConfig {
    /// Load configuration from a YAML file
    pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&contents)?;
        Ok(config)
    }

    /// Load configuration with environment variable overrides
    ///
    /// Environment variables (all optional):
    /// - MPL_LISTEN: Listen address (e.g., "0.0.0.0:9443")
    /// - MPL_UPSTREAM: Upstream server address
    /// - MPL_REGISTRY: Registry path or URL
    /// - MPL_MODE: "transparent" or "strict"
    /// - MPL_PROFILE: QoM profile name
    /// - MPL_ENFORCE_SCHEMA: "true" or "false"
    /// - MPL_ENFORCE_ASSERTIONS: "true" or "false"
    /// - MPL_CONNECT_TIMEOUT_MS: Connection timeout
    /// - MPL_REQUEST_TIMEOUT_MS: Request timeout
    /// - MPL_METRICS_PORT: Metrics server port
    /// - MPL_LOG_LEVEL: Log level (trace, debug, info, warn, error)
    pub fn load_with_env<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let mut config = Self::load(path).unwrap_or_default();
        config.apply_env_overrides();
        Ok(config)
    }

    /// Apply environment variable overrides to configuration
    pub fn apply_env_overrides(&mut self) {
        // Transport settings
        if let Ok(val) = env::var("MPL_LISTEN") {
            self.transport.listen = val;
        }
        if let Ok(val) = env::var("MPL_UPSTREAM") {
            self.transport.upstream = val;
        }
        if let Ok(val) = env::var("MPL_CONNECT_TIMEOUT_MS") {
            if let Ok(ms) = val.parse() {
                self.transport.connect_timeout_ms = ms;
            }
        }
        if let Ok(val) = env::var("MPL_REQUEST_TIMEOUT_MS") {
            if let Ok(ms) = val.parse() {
                self.transport.request_timeout_ms = ms;
            }
        }

        // MPL settings
        if let Ok(val) = env::var("MPL_REGISTRY") {
            self.mpl.registry = val;
        }
        if let Ok(val) = env::var("MPL_MODE") {
            self.mpl.mode = match val.to_lowercase().as_str() {
                "strict" => ProxyMode::Strict,
                _ => ProxyMode::Transparent,
            };
        }
        if let Ok(val) = env::var("MPL_PROFILE") {
            self.mpl.required_profile = Some(val);
        }
        if let Ok(val) = env::var("MPL_ENFORCE_SCHEMA") {
            self.mpl.enforce_schema = val.to_lowercase() == "true";
        }
        if let Ok(val) = env::var("MPL_ENFORCE_ASSERTIONS") {
            self.mpl.enforce_assertions = val.to_lowercase() == "true";
        }

        // Observability settings
        if let Ok(val) = env::var("MPL_METRICS_PORT") {
            if let Ok(port) = val.parse() {
                self.observability.metrics_port = Some(port);
            }
        }
        if let Ok(val) = env::var("MPL_LOG_LEVEL") {
            self.observability.log_level = match val.to_lowercase().as_str() {
                "trace" => LogLevel::Trace,
                "debug" => LogLevel::Debug,
                "warn" => LogLevel::Warn,
                "error" => LogLevel::Error,
                _ => LogLevel::Info,
            };
        }

        // Resource limits
        if let Ok(val) = env::var("MPL_MAX_CONNECTIONS") {
            if let Ok(n) = val.parse() {
                self.limits.max_connections = n;
            }
        }
        if let Ok(val) = env::var("MPL_RATE_LIMIT") {
            if let Ok(n) = val.parse() {
                self.limits.rate_limit_per_second = n;
            }
        }
    }

    /// Save configuration to a YAML file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let contents = serde_yaml::to_string(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }
}

/// Transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    /// Listen address (e.g., "0.0.0.0:9443")
    pub listen: String,

    /// Upstream server address (e.g., "mcp-server:8080")
    pub upstream: String,

    /// Protocol type
    #[serde(default)]
    pub protocol: Protocol,

    /// Connection timeout in milliseconds
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_ms: u64,

    /// Request timeout in milliseconds
    #[serde(default = "default_request_timeout")]
    pub request_timeout_ms: u64,

    /// Idle connection timeout in milliseconds
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_ms: u64,

    /// Maximum number of retries for transient failures
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Maximum request body size in bytes
    #[serde(default = "default_max_body_size")]
    pub max_body_size: usize,
}

fn default_connect_timeout() -> u64 {
    5000 // 5 seconds
}

fn default_request_timeout() -> u64 {
    30000 // 30 seconds
}

fn default_idle_timeout() -> u64 {
    60000 // 60 seconds
}

fn default_max_retries() -> u32 {
    3
}

fn default_max_body_size() -> usize {
    10 * 1024 * 1024 // 10 MB
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            listen: "0.0.0.0:9443".to_string(),
            upstream: "localhost:8080".to_string(),
            protocol: Protocol::Http,
            connect_timeout_ms: default_connect_timeout(),
            request_timeout_ms: default_request_timeout(),
            idle_timeout_ms: default_idle_timeout(),
            max_retries: default_max_retries(),
            max_body_size: default_max_body_size(),
        }
    }
}

impl TransportConfig {
    /// Get connect timeout as Duration
    pub fn connect_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.connect_timeout_ms)
    }

    /// Get request timeout as Duration
    pub fn request_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.request_timeout_ms)
    }

    /// Get idle timeout as Duration
    pub fn idle_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.idle_timeout_ms)
    }
}

/// Supported protocols
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    #[default]
    Http,
    WebSocket,
    Grpc,
}

/// MPL-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MplConfig {
    /// Registry URL
    #[serde(default = "default_registry")]
    pub registry: String,

    /// Proxy mode
    #[serde(default)]
    pub mode: ProxyMode,

    /// Required QoM profile
    pub required_profile: Option<String>,

    /// Enforce schema validation
    #[serde(default = "default_true")]
    pub enforce_schema: bool,

    /// Enforce assertion checks
    #[serde(default = "default_true")]
    pub enforce_assertions: bool,

    /// Enable policy engine
    #[serde(default)]
    pub policy_engine: bool,
}

fn default_registry() -> String {
    "https://github.com/mpl/registry/raw/main".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for MplConfig {
    fn default() -> Self {
        Self {
            registry: default_registry(),
            mode: ProxyMode::Transparent,
            required_profile: Some("qom-basic".to_string()),
            enforce_schema: true,
            enforce_assertions: true,
            policy_engine: false,
        }
    }
}

/// Proxy mode
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyMode {
    /// Log only, don't block invalid requests
    #[default]
    Transparent,
    /// Block requests that fail validation
    Strict,
}

/// Observability configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// Metrics port (Prometheus)
    pub metrics_port: Option<u16>,

    /// Metrics format
    #[serde(default)]
    pub metrics_format: MetricsFormat,

    /// Log output
    #[serde(default)]
    pub logs: LogOutput,

    /// Log format
    #[serde(default)]
    pub log_format: LogFormat,

    /// Log level
    #[serde(default)]
    pub log_level: LogLevel,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            metrics_port: Some(9100),
            metrics_format: MetricsFormat::Prometheus,
            logs: LogOutput::Stdout,
            log_format: LogFormat::Json,
            log_level: LogLevel::Info,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetricsFormat {
    #[default]
    Prometheus,
    OpenTelemetry,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogOutput {
    #[default]
    Stdout,
    Stderr,
    File,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    #[default]
    Json,
    Text,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

/// Route configuration for SType-based routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    /// SType pattern (e.g., "org.calendar.*")
    pub stype_pattern: String,

    /// Target upstream for matching requests
    pub upstream: String,
}

/// Resource limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum concurrent connections
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    /// Maximum requests per second (per client IP)
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_second: u32,

    /// Burst size for rate limiting
    #[serde(default = "default_burst_size")]
    pub burst_size: u32,

    /// Maximum pending requests in queue
    #[serde(default = "default_max_pending")]
    pub max_pending_requests: usize,

    /// Circuit breaker: failure threshold before opening
    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: u32,

    /// Circuit breaker: recovery time in milliseconds
    #[serde(default = "default_recovery_time")]
    pub recovery_time_ms: u64,
}

fn default_max_connections() -> usize {
    10000
}

fn default_rate_limit() -> u32 {
    100
}

fn default_burst_size() -> u32 {
    50
}

fn default_max_pending() -> usize {
    1000
}

fn default_failure_threshold() -> u32 {
    5
}

fn default_recovery_time() -> u64 {
    30000 // 30 seconds
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_connections: default_max_connections(),
            rate_limit_per_second: default_rate_limit(),
            burst_size: default_burst_size(),
            max_pending_requests: default_max_pending(),
            failure_threshold: default_failure_threshold(),
            recovery_time_ms: default_recovery_time(),
        }
    }
}

impl ResourceLimits {
    /// Get recovery time as Duration
    pub fn recovery_time(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.recovery_time_ms)
    }
}
