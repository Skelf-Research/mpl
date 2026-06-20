//! Traffic recording for schema inference
//!
//! Records MCP/A2A traffic samples for schema generation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use tracing::{debug, warn};

/// Traffic record for a single request/response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficRecord {
    /// Unique ID for this record
    pub id: String,
    /// Timestamp of the request
    pub timestamp: String,
    /// Inferred or declared SType
    pub stype: String,
    /// HTTP method
    pub method: String,
    /// Request path
    pub path: String,
    /// Request payload
    pub payload: serde_json::Value,
    /// Response payload (if captured)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<serde_json::Value>,
    /// Response status code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
    /// Request duration in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Whether validation passed
    #[serde(default)]
    pub validation_passed: bool,
    /// Validation errors if any
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub validation_errors: Vec<String>,
}

/// Traffic recorder that stores samples for schema inference
pub struct TrafficRecorder {
    /// Directory to store traffic samples
    data_dir: PathBuf,
    /// In-memory samples by SType (for quick access)
    samples: Arc<RwLock<HashMap<String, Vec<TrafficRecord>>>>,
    /// Counter for unique IDs
    counter: AtomicU64,
    /// Maximum samples to keep per SType
    max_samples_per_stype: usize,
    /// Whether recording is enabled
    enabled: bool,
}

impl TrafficRecorder {
    /// Create a new traffic recorder
    pub fn new(data_dir: &Path, enabled: bool) -> Self {
        let traffic_dir = data_dir.join("traffic");

        // Create traffic directory if it doesn't exist
        if enabled {
            if let Err(e) = fs::create_dir_all(&traffic_dir) {
                warn!("Failed to create traffic directory: {}", e);
            }
        }

        Self {
            data_dir: traffic_dir,
            samples: Arc::new(RwLock::new(HashMap::new())),
            counter: AtomicU64::new(0),
            max_samples_per_stype: 1000,
            enabled,
        }
    }

    /// Check if recording is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Record a traffic sample
    pub fn record(&self, record: TrafficRecord) {
        if !self.enabled {
            return;
        }

        let stype = record.stype.clone();

        // Add to in-memory cache
        if let Ok(mut samples) = self.samples.write() {
            let stype_samples = samples.entry(stype.clone()).or_default();

            // Keep max samples
            if stype_samples.len() >= self.max_samples_per_stype {
                stype_samples.remove(0);
            }

            stype_samples.push(record.clone());
        }

        // Write to disk
        let filename = format!("{}_{}.json", record.stype.replace('.', "_"), record.id);
        let filepath = self.data_dir.join(&filename);

        if let Ok(content) = serde_json::to_string_pretty(&record) {
            if let Err(e) = fs::write(&filepath, content) {
                warn!("Failed to write traffic record: {}", e);
            } else {
                debug!("Recorded traffic sample: {}", filepath.display());
            }
        }
    }

    /// Generate a unique ID for a record
    pub fn next_id(&self) -> String {
        let count = self.counter.fetch_add(1, Ordering::SeqCst);
        format!("{:08x}", count)
    }

    /// Get sample count for an SType
    pub fn sample_count(&self, stype: &str) -> usize {
        self.samples
            .read()
            .ok()
            .and_then(|s| s.get(stype).map(|v| v.len()))
            .unwrap_or(0)
    }

    /// Get all STypes with sample counts
    pub fn get_stats(&self) -> HashMap<String, usize> {
        self.samples
            .read()
            .ok()
            .map(|s| s.iter().map(|(k, v)| (k.clone(), v.len())).collect())
            .unwrap_or_default()
    }

    /// Get samples for an SType
    pub fn get_samples(&self, stype: &str) -> Vec<TrafficRecord> {
        self.samples
            .read()
            .ok()
            .and_then(|s| s.get(stype).cloned())
            .unwrap_or_default()
    }

    /// Load samples from disk
    pub fn load_from_disk(&self) -> anyhow::Result<usize> {
        if !self.data_dir.exists() {
            return Ok(0);
        }

        let mut loaded = 0;

        for entry in fs::read_dir(&self.data_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(record) = serde_json::from_str::<TrafficRecord>(&content) {
                        if let Ok(mut samples) = self.samples.write() {
                            samples
                                .entry(record.stype.clone())
                                .or_default()
                                .push(record);
                            loaded += 1;
                        }
                    }
                }
            }
        }

        debug!("Loaded {} traffic samples from disk", loaded);
        Ok(loaded)
    }
}

/// SType inference from payload structure
pub struct StypeInferrer;

impl StypeInferrer {
    /// Infer an SType from a JSON payload and request context
    pub fn infer(path: &str, method: &str, payload: &serde_json::Value) -> String {
        // Check for A2A task patterns first
        if let Some(stype) = Self::infer_a2a(path, payload) {
            return stype;
        }

        // Try to extract from JSON-RPC method
        if let Some(rpc_method) = payload.get("method").and_then(|m| m.as_str()) {
            return Self::method_to_stype(rpc_method);
        }

        // Try to extract from MCP tools/call
        if let Some(params) = payload.get("params") {
            if let Some(name) = params.get("name").and_then(|n| n.as_str()) {
                return format!("inferred.tool.{}.v1", Self::normalize_name(name));
            }
        }

        // Infer from path
        let path_parts: Vec<&str> = path
            .trim_matches('/')
            .split('/')
            .filter(|p| !p.is_empty())
            .collect();

        if !path_parts.is_empty() {
            let name = path_parts.last().unwrap_or(&"unknown");
            return format!(
                "inferred.{}.{}.v1",
                method.to_lowercase(),
                Self::normalize_name(name)
            );
        }

        // Fallback: hash-based inference
        "inferred.unknown.payload.v1".to_string()
    }

    /// Infer SType for A2A protocol patterns
    fn infer_a2a(path: &str, payload: &serde_json::Value) -> Option<String> {
        // A2A task endpoints: /tasks, /tasks/{id}, /tasks/{id}/send, etc.
        if path.contains("/tasks") {
            // Check for task_id in payload or path
            if let Some(task_id) = payload.get("task_id").or(payload.get("id")) {
                if task_id.is_string() {
                    // Determine operation from path
                    if path.contains("/send") {
                        return Some("a2a.task.SendMessage.v1".to_string());
                    } else if path.contains("/cancel") {
                        return Some("a2a.task.Cancel.v1".to_string());
                    } else if path.ends_with("/tasks") || path.contains("/tasks/") {
                        return Some("a2a.task.Task.v1".to_string());
                    }
                }
            }

            // A2A message pattern
            if payload.get("message").is_some() || payload.get("messages").is_some() {
                return Some("a2a.task.Message.v1".to_string());
            }

            return Some("a2a.task.Request.v1".to_string());
        }

        // A2A agent info endpoint
        if path.contains("/agent") || path.contains("/.well-known/agent") {
            return Some("a2a.agent.Info.v1".to_string());
        }

        // A2A streaming/push notifications
        if path.contains("/subscribe") || path.contains("/notifications") {
            return Some("a2a.notification.Subscribe.v1".to_string());
        }

        None
    }

    /// Convert JSON-RPC method to SType
    fn method_to_stype(method: &str) -> String {
        // e.g., "tools/call" -> "mcp.tools.call.v1"
        let normalized = method.replace('/', ".");
        format!("mcp.{}.v1", normalized)
    }

    /// Normalize a name for use in SType
    fn normalize_name(name: &str) -> String {
        name.chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect::<String>()
            .to_lowercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_infer_from_jsonrpc() {
        let payload = json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {"name": "test"}
        });

        let stype = StypeInferrer::infer("/", "POST", &payload);
        assert_eq!(stype, "mcp.tools.call.v1");
    }

    #[test]
    fn test_infer_from_tool_name() {
        let payload = json!({
            "jsonrpc": "2.0",
            "params": {"name": "calendar_create"}
        });

        let stype = StypeInferrer::infer("/", "POST", &payload);
        assert_eq!(stype, "inferred.tool.calendar_create.v1");
    }

    #[test]
    fn test_infer_from_path() {
        let payload = json!({});
        let stype = StypeInferrer::infer("/api/events", "POST", &payload);
        assert_eq!(stype, "inferred.post.events.v1");
    }
}
