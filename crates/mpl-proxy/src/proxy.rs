//! Core proxy logic

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use mpl_core::assertions::{AssertionSet, EvaluationContext};
use mpl_core::envelope::MplEnvelope;
use mpl_core::hash::{semantic_hash, verify_hash};
use mpl_core::metrics::{TocMethod, TocResult};
use mpl_core::ontology::Ontology;
use mpl_core::qom::{QomMetrics, QomProfile};
use mpl_core::validation::SchemaValidator;

use crate::config::{ProxyConfig, ProxyMode};
use crate::metrics::MetricsState;
use crate::qom_recorder::{QomRecorder, QomRecorderConfig, QomScores};
use crate::traffic::TrafficRecorder;

/// MPL headers
pub const HEADER_STYPE: &str = "X-MPL-SType";
pub const HEADER_PROFILE: &str = "X-MPL-Profile";
pub const HEADER_SEM_HASH: &str = "X-MPL-Sem-Hash";
pub const HEADER_QOM_RESULT: &str = "X-MPL-QoM-Result";
/// TOC verification result header (values: "verified", "failed", "pending", "skip")
pub const HEADER_TOC_RESULT: &str = "X-MPL-TOC-Result";
/// TOC verification callback ID (for async verification)
pub const HEADER_TOC_CALLBACK_ID: &str = "X-MPL-TOC-Callback-Id";

/// Validation result for a request
#[derive(Debug, Clone, Serialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub stype: Option<String>,
    pub schema_valid: bool,
    pub qom_passed: bool,
    pub hash_valid: bool,
    /// TOC result if available (from header or callback)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toc_result: Option<TocResult>,
    /// IC score if computed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ic_score: Option<f64>,
    /// Profile used for evaluation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_used: Option<String>,
    /// Whether profile was degraded from original
    #[serde(default)]
    pub degraded: bool,
    /// Original profile before degradation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_profile: Option<String>,
    /// Retry count (0 = first attempt)
    #[serde(default)]
    pub retry_count: u32,
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
            toc_result: None,
            ic_score: None,
            profile_used: None,
            degraded: false,
            original_profile: None,
            retry_count: 0,
            errors: Vec::new(),
        }
    }
}

/// Pending TOC verification tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingTocVerification {
    /// Unique callback ID
    pub callback_id: String,
    /// SType being verified
    pub stype: String,
    /// Request timestamp
    pub timestamp: String,
    /// Expected outcome (if specified)
    pub expected_outcome: Option<String>,
    /// Tool name (if applicable)
    pub tool_name: Option<String>,
}

/// Shared proxy state
pub struct ProxyState {
    /// Configuration
    pub config: ProxyConfig,

    /// HTTP client for upstream requests
    pub client: Client,

    /// Schema validator
    pub validator: SchemaValidator,

    /// Assertions by SType (for IC computation)
    pub assertions: Arc<RwLock<HashMap<String, AssertionSet>>>,

    /// QoM profiles
    pub profiles: Vec<QomProfile>,

    /// Metrics
    pub metrics: Arc<MetricsState>,

    /// QoM recorder for full metric tracking and persistence
    pub qom_recorder: Arc<QomRecorder>,

    /// Traffic recorder for schema inference
    pub traffic_recorder: Arc<TrafficRecorder>,

    /// Pending TOC verifications (callback_id -> verification)
    pub pending_toc: Arc<RwLock<HashMap<String, PendingTocVerification>>>,

    /// Completed TOC results (callback_id -> result)
    pub completed_toc: Arc<RwLock<HashMap<String, TocResult>>>,

    /// Counter for generating callback IDs
    toc_counter: std::sync::atomic::AtomicU64,
}

impl ProxyState {
    /// Create a new proxy state from configuration
    pub async fn new(config: ProxyConfig) -> Result<Self> {
        Self::with_options(config, None, false).await
    }

    /// Create a new proxy state with traffic recording options
    pub async fn with_options(
        config: ProxyConfig,
        data_dir: Option<&str>,
        learning_enabled: bool,
    ) -> Result<Self> {
        // Build HTTP client with configured timeouts
        let client = Client::builder()
            .connect_timeout(config.transport.connect_timeout())
            .timeout(config.transport.request_timeout())
            .pool_idle_timeout(config.transport.idle_timeout())
            .build()?;

        let mut validator = SchemaValidator::new();
        let mut assertions_map: HashMap<String, AssertionSet> = HashMap::new();

        // Load schemas and assertions from registry if it's a local path
        let registry_path = &config.mpl.registry;
        if Path::new(registry_path).exists() {
            Self::load_schemas_from_registry(&mut validator, registry_path)?;
            Self::load_assertions_from_registry(&mut assertions_map, registry_path)?;
        }

        let profiles = vec![
            QomProfile::basic(),
            QomProfile::strict_argcheck(),
            QomProfile::outcome(),
            QomProfile::comprehensive(),
        ];

        let metrics = Arc::new(MetricsState::new());

        // Initialize traffic recorder. Expand `~/` so a literal `~/.mpl/...`
        // directory isn't created wherever the binary was invoked.
        let data_path = match data_dir {
            Some(p) => mpl_core::util::expand_tilde(p),
            None => mpl_core::util::expand_tilde("~/.mpl"),
        };
        let traffic_recorder = Arc::new(TrafficRecorder::new(&data_path, learning_enabled));

        if learning_enabled {
            // Load existing samples from disk
            if let Err(e) = traffic_recorder.load_from_disk() {
                warn!("Failed to load existing traffic samples: {}", e);
            }
            info!("Traffic recording enabled");
        }

        // Initialize QoM recorder
        let qom_data_dir = data_path.join("qom");
        let qom_recorder = Arc::new(QomRecorder::new(QomRecorderConfig {
            data_dir: qom_data_dir,
            ..Default::default()
        }));

        // Load QoM events from disk
        if let Err(e) = qom_recorder.load_from_disk().await {
            warn!("Failed to load QoM events: {}", e);
        }

        // Load ontology specs from registry
        Self::load_ontologies_from_registry(&qom_recorder, registry_path).await;

        info!("Proxy state initialized");
        info!("Mode: {:?}", config.mpl.mode);
        info!("Loaded {} schemas", validator.registered_stypes().len());
        info!("Loaded {} assertion sets", assertions_map.len());
        info!("Loaded {} QoM profiles", profiles.len());

        Ok(Self {
            config,
            client,
            validator,
            assertions: Arc::new(RwLock::new(assertions_map)),
            profiles,
            metrics,
            qom_recorder,
            traffic_recorder,
            pending_toc: Arc::new(RwLock::new(HashMap::new())),
            completed_toc: Arc::new(RwLock::new(HashMap::new())),
            toc_counter: std::sync::atomic::AtomicU64::new(0),
        })
    }

    /// Generate a unique TOC callback ID
    pub fn next_toc_callback_id(&self) -> String {
        let count = self
            .toc_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        format!("toc-{:016x}", count)
    }

    /// Register a pending TOC verification
    pub fn register_pending_toc(&self, verification: PendingTocVerification) {
        if let Ok(mut pending) = self.pending_toc.write() {
            pending.insert(verification.callback_id.clone(), verification);
        }
    }

    /// Complete a TOC verification (called from callback endpoint)
    pub fn complete_toc(&self, callback_id: &str, result: TocResult) -> bool {
        // Remove from pending
        let mut was_pending = false;
        if let Ok(mut pending) = self.pending_toc.write() {
            was_pending = pending.remove(callback_id).is_some();
        }

        // Add to completed
        if let Ok(mut completed) = self.completed_toc.write() {
            completed.insert(callback_id.to_string(), result);
        }

        was_pending
    }

    /// Get completed TOC result for a callback ID
    pub fn get_toc_result(&self, callback_id: &str) -> Option<TocResult> {
        self.completed_toc
            .read()
            .ok()
            .and_then(|completed| completed.get(callback_id).cloned())
    }

    /// Get pending TOC verification
    pub fn get_pending_toc(&self, callback_id: &str) -> Option<PendingTocVerification> {
        self.pending_toc
            .read()
            .ok()
            .and_then(|pending| pending.get(callback_id).cloned())
    }

    /// Parse TOC result from header value
    pub fn parse_toc_header(value: &str) -> Option<TocResult> {
        match value.to_lowercase().as_str() {
            "verified" | "pass" | "true" | "1" => Some(TocResult::verified(TocMethod::Header)),
            "failed" | "fail" | "false" | "0" => {
                Some(TocResult::failed(TocMethod::Header, "Verification failed"))
            }
            "pending" => None, // Still waiting
            "skip" | "na" => Some(TocResult::verified(TocMethod::None)), // Not applicable
            _ => None,
        }
    }

    /// Load schemas from local registry directory
    fn load_schemas_from_registry(
        validator: &mut SchemaValidator,
        registry_path: &str,
    ) -> Result<()> {
        let stypes_path = Path::new(registry_path).join("stypes");
        if !stypes_path.exists() {
            debug!(
                "Registry stypes path does not exist: {}",
                stypes_path.display()
            );
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
                        let stype = format!("{}.{}.{}", namespace, name, version_str);

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

    /// Load assertions from local registry directory
    fn load_assertions_from_registry(
        assertions: &mut HashMap<String, AssertionSet>,
        registry_path: &str,
    ) -> Result<()> {
        let stypes_path = Path::new(registry_path).join("stypes");
        if !stypes_path.exists() {
            return Ok(());
        }

        // Walk the stypes directory structure looking for assertions.json files
        Self::walk_registry_dir_for_assertions(assertions, &stypes_path, Vec::new())?;
        Ok(())
    }

    fn walk_registry_dir_for_assertions(
        assertions: &mut HashMap<String, AssertionSet>,
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
                Self::walk_registry_dir_for_assertions(assertions, &entry_path, new_parts)?;
            } else if name == "assertions.json" && parts.len() >= 4 {
                // We have: namespace/domain/Name/vN/assertions.json
                let version_str = &parts[parts.len() - 1];
                if let Some(version) = version_str.strip_prefix('v') {
                    if version.parse::<u32>().is_ok() {
                        let namespace = parts[..parts.len() - 2].join(".");
                        let type_name = &parts[parts.len() - 2];
                        let stype = format!("{}.{}.{}", namespace, type_name, version_str);

                        // Read and parse assertions
                        if let Ok(content) = std::fs::read_to_string(&entry_path) {
                            match serde_json::from_str::<AssertionSet>(&content) {
                                Ok(assertion_set) => {
                                    debug!(
                                        "Loaded {} assertions for {}",
                                        assertion_set.assertions.len(),
                                        stype
                                    );
                                    assertions.insert(stype, assertion_set);
                                }
                                Err(e) => {
                                    warn!(
                                        "Failed to parse assertions for {}: {}",
                                        entry_path.display(),
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Load ontology specs from local registry directory
    async fn load_ontologies_from_registry(qom_recorder: &QomRecorder, registry_path: &str) {
        let stypes_path = Path::new(registry_path).join("stypes");
        if !stypes_path.exists() {
            return;
        }

        // Collect all ontologies synchronously first
        let ontologies = Self::collect_ontologies_from_registry(&stypes_path, Vec::new());

        // Then load them asynchronously
        for (stype, spec) in ontologies {
            debug!("Loaded ontology for {}", stype);
            qom_recorder.load_ontology(&stype, spec).await;
        }
    }

    fn collect_ontologies_from_registry(
        path: &Path,
        parts: Vec<String>,
    ) -> Vec<(String, Ontology)> {
        let mut result = Vec::new();

        if !path.is_dir() {
            return result;
        }

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                if entry_path.is_dir() {
                    let mut new_parts = parts.clone();
                    new_parts.push(name);
                    result.extend(Self::collect_ontologies_from_registry(
                        &entry_path,
                        new_parts,
                    ));
                } else if name == "ontology.json" && parts.len() >= 4 {
                    // We have: namespace/domain/Name/vN/ontology.json
                    let version_str = &parts[parts.len() - 1];
                    if let Some(version) = version_str.strip_prefix('v') {
                        if version.parse::<u32>().is_ok() {
                            let namespace = parts[..parts.len() - 2].join(".");
                            let type_name = &parts[parts.len() - 2];
                            let stype = format!("{}.{}.{}", namespace, type_name, version_str);

                            // Read and parse ontology spec
                            if let Ok(content) = std::fs::read_to_string(&entry_path) {
                                match serde_json::from_str::<Ontology>(&content) {
                                    Ok(spec) => {
                                        result.push((stype, spec));
                                    }
                                    Err(e) => {
                                        warn!(
                                            "Failed to parse ontology for {}: {}",
                                            entry_path.display(),
                                            e
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        result
    }

    /// Get assertions for an SType
    pub fn get_assertions(&self, stype: &str) -> Option<AssertionSet> {
        self.assertions
            .read()
            .ok()
            .and_then(|a| a.get(stype).cloned())
    }

    /// Validate an MPL request
    pub async fn validate_request(&self, envelope: &MplEnvelope) -> ValidationResult {
        self.validate_request_full(envelope, None).await
    }

    /// Validate an MPL request with optional response for determinism checking
    pub async fn validate_request_full(
        &self,
        envelope: &MplEnvelope,
        response: Option<&serde_json::Value>,
    ) -> ValidationResult {
        let mut result = ValidationResult {
            stype: Some(envelope.stype.clone()),
            ..Default::default()
        };

        // Compute payload hash for determinism tracking
        let payload_hash = semantic_hash(&envelope.payload).ok();

        // Schema validation
        let sf_score = if self.config.mpl.enforce_schema {
            match self.validator.validate(&envelope.stype, &envelope.payload) {
                Ok(validation) => {
                    result.schema_valid = validation.valid;
                    if !validation.valid {
                        result.valid = false;
                        for err in validation.errors {
                            result
                                .errors
                                .push(format!("Schema error at {}: {}", err.path, err.message));
                        }
                    }
                    if validation.valid {
                        1.0
                    } else {
                        0.0
                    }
                }
                Err(e) => {
                    // Unknown SType - check mode
                    if self.is_strict() {
                        result.valid = false;
                        result.schema_valid = false;
                        result
                            .errors
                            .push(format!("Unknown SType: {} ({})", envelope.stype, e));
                        0.0
                    } else {
                        warn!(
                            "Unknown SType: {}, allowing in transparent mode",
                            envelope.stype
                        );
                        1.0
                    }
                }
            }
        } else {
            1.0
        };

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
                    result
                        .errors
                        .push(format!("Hash verification failed: {}", e));
                }
            }
        }

        // Instruction Compliance (IC) - evaluate assertions if available
        let ic_score = if let Some(assertion_set) = self.get_assertions(&envelope.stype) {
            let eval_ctx = EvaluationContext {
                stype: Some(envelope.stype.clone()),
                ..Default::default()
            };

            match assertion_set.evaluate_with_context(&envelope.payload, &eval_ctx) {
                Ok(assertion_result) => {
                    let score = assertion_result.ic_score;
                    result.ic_score = Some(score);

                    // Add assertion failures to errors
                    for ar in &assertion_result.results {
                        if !ar.passed {
                            match ar.severity {
                                mpl_core::assertions::AssertionSeverity::Error => {
                                    result
                                        .errors
                                        .push(format!("IC error [{}]: {}", ar.id, ar.message));
                                }
                                mpl_core::assertions::AssertionSeverity::Warning => {
                                    debug!("IC warning [{}]: {}", ar.id, ar.message);
                                }
                                mpl_core::assertions::AssertionSeverity::Info => {
                                    debug!("IC info [{}]: {}", ar.id, ar.message);
                                }
                            }
                        }
                    }

                    // Check if any error-severity assertions failed
                    if assertion_result.error_count > 0 && self.is_strict() {
                        result.valid = false;
                    }

                    Some(score)
                }
                Err(e) => {
                    warn!("Assertion evaluation failed for {}: {}", envelope.stype, e);
                    None
                }
            }
        } else {
            None
        };

        // Ontology Adherence (OA) - check domain constraints
        let oa_result = self
            .qom_recorder
            .check_ontology(&envelope.stype, &envelope.payload)
            .await;
        let oa_score = if oa_result.constraints_checked > 0 {
            Some(oa_result.score)
        } else {
            None
        };

        // Determinism Jitter (DJ) - check response stability if response provided
        let dj_score = if let (Some(resp), Some(ref hash)) = (response, &payload_hash) {
            let dj_result = self
                .qom_recorder
                .check_determinism(&envelope.stype, hash, resp)
                .await;
            if dj_result.comparison_count > 0 {
                Some(dj_result.similarity)
            } else {
                None
            }
        } else {
            None
        };

        // QoM evaluation - now includes all computed metrics
        let profile_name = if let Some(profile) = self.active_profile() {
            let mut metrics = if result.schema_valid {
                QomMetrics::schema_valid()
            } else {
                QomMetrics::schema_invalid()
            };

            // Add all computed metrics
            if let Some(ic) = ic_score {
                metrics = metrics.with_instruction_compliance(ic);
            }
            if let Some(oa) = oa_score {
                metrics = metrics.with_ontology_adherence(oa);
            }
            if let Some(dj) = dj_score {
                metrics = metrics.with_determinism_jitter(dj);
            }

            let evaluation = profile.evaluate(&metrics);
            result.qom_passed = evaluation.meets_profile;
            result.profile_used = Some(profile.name.clone());

            if !evaluation.meets_profile {
                result.valid = false;
                for failure in evaluation.failures {
                    result.errors.push(format!(
                        "QoM breach: {} < {}",
                        failure.metric, failure.threshold
                    ));
                }
            }

            Some(profile.name.clone())
        } else {
            None
        };

        // Update metrics counters
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

        // Record QoM event
        let scores = QomScores {
            sf: Some(sf_score),
            ic: ic_score,
            toc: None, // TOC is async, handled separately
            g: None,   // Groundedness requires response content analysis
            dj: dj_score,
            oa: oa_score,
        };

        let failure_reason = if !result.qom_passed && !result.errors.is_empty() {
            Some(result.errors.join("; "))
        } else {
            None
        };

        let event = self.qom_recorder.create_event(
            &envelope.stype,
            &profile_name.unwrap_or_else(|| "none".to_string()),
            result.qom_passed,
            scores,
            failure_reason,
            payload_hash,
        );
        self.qom_recorder.record_event(event).await;

        result
    }

    /// Forward a request to the upstream server
    pub async fn forward_request(
        &self,
        path: String,
        request: Request<Body>,
    ) -> Result<Response<Body>> {
        use crate::traffic::{StypeInferrer, TrafficRecord};

        let start_time = std::time::Instant::now();
        let upstream = &self.config.transport.upstream;
        let uri = format!("http://{}/{}", upstream, path);

        debug!("Forwarding to: {}", uri);

        // Extract headers
        let method = request.method().clone();
        let method_str = method.to_string();
        let headers = request.headers().clone();
        let stype_header = headers
            .get(HEADER_STYPE)
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        // Read body
        let body_bytes = axum::body::to_bytes(request.into_body(), usize::MAX).await?;

        // Parse payload for traffic recording
        let payload: serde_json::Value =
            serde_json::from_slice(&body_bytes).unwrap_or(serde_json::Value::Null);

        // Try to parse as MPL envelope or create one from headers
        let envelope = if let Ok(env) = serde_json::from_slice::<MplEnvelope>(&body_bytes) {
            Some(env)
        } else if let Some(stype) = stype_header.clone() {
            // Create envelope from headers + body
            if let Ok(payload) = serde_json::from_slice(&body_bytes) {
                Some(MplEnvelope::new(stype, payload))
            } else {
                None
            }
        } else {
            None
        };

        // Determine SType (from envelope, header, or inferred)
        let stype = envelope
            .as_ref()
            .map(|e| e.stype.clone())
            .or(stype_header)
            .unwrap_or_else(|| StypeInferrer::infer(&path, &method_str, &payload));

        // Validate if we have an envelope
        let validation_result = if let Some(ref env) = envelope {
            let result = self.validate_request(env).await;

            // In strict mode, block invalid requests
            if !result.valid && self.is_strict() {
                let error_response = serde_json::json!({
                    "error": "E-SCHEMA-FIDELITY",
                    "message": "Request validation failed",
                    "details": result.errors,
                });

                // Record failed request if learning is enabled
                if self.traffic_recorder.is_enabled() {
                    let record = TrafficRecord {
                        id: self.traffic_recorder.next_id(),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        stype: stype.clone(),
                        method: method_str.clone(),
                        path: path.clone(),
                        payload: payload.clone(),
                        response: Some(error_response.clone()),
                        status_code: Some(400),
                        duration_ms: Some(start_time.elapsed().as_millis() as u64),
                        validation_passed: false,
                        validation_errors: result.errors.clone(),
                    };
                    self.traffic_recorder.record(record);
                }

                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("content-type", "application/json")
                    .header(
                        HEADER_QOM_RESULT,
                        if result.qom_passed { "pass" } else { "fail" },
                    )
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
        let status_code = status.as_u16();
        let response_headers = upstream_response.headers().clone();
        let body = upstream_response.bytes().await?;

        // Record traffic if learning is enabled
        if self.traffic_recorder.is_enabled() {
            let response_payload: Option<serde_json::Value> = serde_json::from_slice(&body).ok();

            let record = TrafficRecord {
                id: self.traffic_recorder.next_id(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                stype,
                method: method_str,
                path,
                payload,
                response: response_payload,
                status_code: Some(status_code),
                duration_ms: Some(start_time.elapsed().as_millis() as u64),
                validation_passed: validation_result.as_ref().map(|r| r.valid).unwrap_or(true),
                validation_errors: validation_result
                    .as_ref()
                    .map(|r| r.errors.clone())
                    .unwrap_or_default(),
            };
            self.traffic_recorder.record(record);
        }

        let mut response = Response::builder().status(status);

        for (name, value) in response_headers.iter() {
            response = response.header(name, value);
        }

        // Add MPL headers to response
        if let Some(ref result) = validation_result {
            response = response.header(
                HEADER_QOM_RESULT,
                if result.qom_passed { "pass" } else { "fail" },
            );
        }

        Ok(response.body(Body::from(body))?)
    }

    /// Get the active QoM profile
    pub fn active_profile(&self) -> Option<&QomProfile> {
        let profile_name = self.config.mpl.required_profile.as_ref()?;
        self.profiles.iter().find(|p| &p.name == profile_name)
    }

    /// Get a profile by name
    pub fn get_profile(&self, name: &str) -> Option<&QomProfile> {
        self.profiles.iter().find(|p| p.name == name)
    }

    /// Get the degradation chain for a profile
    /// Returns profiles from strictest to most lenient
    pub fn get_degradation_chain(&self, start_profile: &str) -> Vec<&QomProfile> {
        // Profile degradation order: comprehensive -> outcome -> strict-argcheck -> basic
        let order = [
            "qom-comprehensive",
            "qom-outcome",
            "qom-strict-argcheck",
            "qom-basic",
        ];

        let start_idx = order.iter().position(|&p| p == start_profile).unwrap_or(0);

        order[start_idx..]
            .iter()
            .filter_map(|&name| self.get_profile(name))
            .collect()
    }

    /// Validate with automatic profile degradation
    /// Returns (result, final_profile_name, was_degraded)
    pub async fn validate_with_degradation(&self, envelope: &MplEnvelope) -> ValidationResult {
        let original_profile = self.config.mpl.required_profile.clone();

        if let Some(ref profile_name) = original_profile {
            let chain = self.get_degradation_chain(profile_name);

            for (idx, profile) in chain.iter().enumerate() {
                let mut result = self.validate_request_with_profile(envelope, profile);

                if result.qom_passed || idx == chain.len() - 1 {
                    // Either passed or we're at the end of the chain
                    result.profile_used = Some(profile.name.clone());
                    result.degraded = idx > 0;
                    if idx > 0 {
                        result.original_profile = Some(profile_name.clone());
                    }
                    return result;
                }
            }
        }

        // No profile configured, just validate
        self.validate_request(envelope).await
    }

    /// Validate with a specific profile (internal)
    fn validate_request_with_profile(
        &self,
        envelope: &MplEnvelope,
        profile: &QomProfile,
    ) -> ValidationResult {
        let mut result = ValidationResult {
            stype: Some(envelope.stype.clone()),
            profile_used: Some(profile.name.clone()),
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
                            result
                                .errors
                                .push(format!("Schema error at {}: {}", err.path, err.message));
                        }
                    }
                }
                Err(e) => {
                    if self.is_strict() {
                        result.valid = false;
                        result.schema_valid = false;
                        result
                            .errors
                            .push(format!("Unknown SType: {} ({})", envelope.stype, e));
                    }
                }
            }
        }

        // IC evaluation
        let ic_score = if let Some(assertion_set) = self.get_assertions(&envelope.stype) {
            let eval_ctx = EvaluationContext {
                stype: Some(envelope.stype.clone()),
                ..Default::default()
            };

            match assertion_set.evaluate_with_context(&envelope.payload, &eval_ctx) {
                Ok(assertion_result) => {
                    result.ic_score = Some(assertion_result.ic_score);
                    Some(assertion_result.ic_score)
                }
                Err(_) => None,
            }
        } else {
            None
        };

        // Build metrics and evaluate against profile
        let mut metrics = if result.schema_valid {
            QomMetrics::schema_valid()
        } else {
            QomMetrics::schema_invalid()
        };

        if let Some(ic) = ic_score {
            metrics = metrics.with_instruction_compliance(ic);
        }

        let evaluation = profile.evaluate(&metrics);
        result.qom_passed = evaluation.meets_profile;

        if !evaluation.meets_profile {
            for failure in evaluation.failures {
                result.errors.push(format!(
                    "QoM breach [{}]: {} < {}",
                    profile.name, failure.metric, failure.threshold
                ));
            }
        }

        result
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
        let server_stypes: Vec<String> = self
            .validator
            .registered_stypes()
            .iter()
            .map(|s| s.to_string())
            .collect();
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
