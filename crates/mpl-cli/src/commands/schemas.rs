//! Schema management commands
//!
//! Commands for schema inference, approval, and management.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use sha2::{Sha256, Digest};

use crate::SchemaStatus;

/// Export mode for delta updates
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportMode {
    /// Only export new schemas (skip existing)
    New,
    /// Export new and changed schemas
    Delta,
    /// Overwrite all schemas
    Full,
    /// Bump version for changed schemas
    BumpVersion,
}

impl std::str::FromStr for ExportMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "new" => Ok(ExportMode::New),
            "delta" => Ok(ExportMode::Delta),
            "full" => Ok(ExportMode::Full),
            "bump" | "bump-version" => Ok(ExportMode::BumpVersion),
            _ => Err(format!("Unknown export mode: {}", s)),
        }
    }
}

/// Compute hash of a schema for change detection
fn compute_schema_hash(schema: &serde_json::Value) -> String {
    let canonical = serde_json::to_string(schema).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Schema metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaInfo {
    pub stype: String,
    pub status: String,
    pub sample_count: usize,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub schema: serde_json::Value,
    /// Hash of the schema for change detection
    #[serde(default)]
    pub schema_hash: Option<String>,
    /// When this schema was last exported to registry
    #[serde(default)]
    pub exported_at: Option<String>,
    /// Hash at time of last export (for delta detection)
    #[serde(default)]
    pub exported_hash: Option<String>,
}

/// Schema inference state
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct InferenceState {
    pub schemas: HashMap<String, SchemaInfo>,
    pub samples: HashMap<String, Vec<serde_json::Value>>,
}

impl InferenceState {
    pub fn load(path: &Path) -> Result<Self> {
        let state_file = path.join("inference_state.json");
        if state_file.exists() {
            let content = fs::read_to_string(&state_file)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        fs::create_dir_all(path)?;
        let state_file = path.join("inference_state.json");
        let content = serde_json::to_string_pretty(self)?;
        fs::write(state_file, content)?;
        Ok(())
    }
}

/// Generate schemas from recorded traffic
pub fn generate(data_dir: &str, min_samples: usize, output: &str) -> Result<()> {
    let data_path = Path::new(data_dir);
    let traffic_path = data_path.join("traffic");
    let output_path = Path::new(output);

    // Load existing inference state
    let mut state = InferenceState::load(output_path).unwrap_or_default();

    // Load recorded traffic samples
    if traffic_path.exists() {
        load_traffic_samples(&traffic_path, &mut state)?;
    }

    // Generate schemas for stypes with enough samples
    let mut generated = 0;
    for (stype, samples) in &state.samples {
        if samples.len() >= min_samples {
            if let Ok(schema) = infer_schema(samples) {
                let schema_hash = compute_schema_hash(&schema);
                let now = chrono::Utc::now().to_rfc3339();

                // Preserve existing export tracking info if schema exists
                let (created_at, exported_at, exported_hash) =
                    if let Some(existing) = state.schemas.get(stype) {
                        (
                            existing.created_at.clone(),
                            existing.exported_at.clone(),
                            existing.exported_hash.clone(),
                        )
                    } else {
                        (now.clone(), None, None)
                    };

                let info = SchemaInfo {
                    stype: stype.clone(),
                    status: "pending".to_string(),
                    sample_count: samples.len(),
                    created_at,
                    updated_at: now,
                    schema,
                    schema_hash: Some(schema_hash),
                    exported_at,
                    exported_hash,
                };
                state.schemas.insert(stype.clone(), info);
                generated += 1;
            }
        }
    }

    // Save state
    state.save(output_path)?;

    // Write individual schema files
    fs::create_dir_all(output_path)?;
    for (stype, info) in &state.schemas {
        let filename = format!("{}.json", stype.replace('.', "_"));
        let schema_file = output_path.join(&filename);
        let content = serde_json::to_string_pretty(&info.schema)?;
        fs::write(&schema_file, content)?;
    }

    println!("Generated {} schemas from {} stypes", generated, state.samples.len());
    println!("Output: {}", output);

    if state.samples.iter().any(|(_, s)| s.len() < min_samples) {
        println!();
        println!("Note: Some stypes have fewer than {} samples:", min_samples);
        for (stype, samples) in &state.samples {
            if samples.len() < min_samples {
                println!("  - {} ({} samples)", stype, samples.len());
            }
        }
    }

    Ok(())
}

/// Load traffic samples from recorded files
fn load_traffic_samples(traffic_path: &Path, state: &mut InferenceState) -> Result<()> {
    for entry in fs::read_dir(traffic_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "json").unwrap_or(false) {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(record) = serde_json::from_str::<TrafficRecord>(&content) {
                    state
                        .samples
                        .entry(record.stype.clone())
                        .or_default()
                        .push(record.payload);
                }
            }
        }
    }
    Ok(())
}

/// Traffic record from recorded samples
#[derive(Debug, Deserialize)]
struct TrafficRecord {
    stype: String,
    payload: serde_json::Value,
}

/// Infer JSON schema from sample payloads
fn infer_schema(samples: &[serde_json::Value]) -> Result<serde_json::Value> {
    if samples.is_empty() {
        anyhow::bail!("No samples to infer from");
    }

    // Start with the first sample
    let mut schema = infer_from_value(&samples[0]);

    // Merge with subsequent samples to find common structure
    for sample in samples.iter().skip(1) {
        let sample_schema = infer_from_value(sample);
        schema = merge_schemas(schema, sample_schema);
    }

    Ok(serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": schema.get("type").unwrap_or(&serde_json::json!("object")),
        "properties": schema.get("properties").unwrap_or(&serde_json::json!({})),
        "required": schema.get("required").unwrap_or(&serde_json::json!([]))
    }))
}

/// Infer schema structure from a single value
fn infer_from_value(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Null => serde_json::json!({"type": "null"}),
        serde_json::Value::Bool(_) => serde_json::json!({"type": "boolean"}),
        serde_json::Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                serde_json::json!({"type": "integer"})
            } else {
                serde_json::json!({"type": "number"})
            }
        }
        serde_json::Value::String(_) => serde_json::json!({"type": "string"}),
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                serde_json::json!({"type": "array", "items": {}})
            } else {
                let item_schema = infer_from_value(&arr[0]);
                serde_json::json!({"type": "array", "items": item_schema})
            }
        }
        serde_json::Value::Object(obj) => {
            let mut properties = serde_json::Map::new();
            let mut required = Vec::new();

            for (key, val) in obj {
                properties.insert(key.clone(), infer_from_value(val));
                required.push(serde_json::Value::String(key.clone()));
            }

            serde_json::json!({
                "type": "object",
                "properties": properties,
                "required": required
            })
        }
    }
}

/// Merge two schemas, keeping common structure
fn merge_schemas(a: serde_json::Value, b: serde_json::Value) -> serde_json::Value {
    // Simple merge - keep type from a, merge properties
    let a_type = a.get("type").cloned().unwrap_or(serde_json::json!("object"));
    let b_type = b.get("type").cloned().unwrap_or(serde_json::json!("object"));

    if a_type != b_type {
        // Types differ, make it a union
        return serde_json::json!({
            "oneOf": [a, b]
        });
    }

    if a_type == "object" {
        let a_props = a.get("properties").and_then(|p| p.as_object());
        let b_props = b.get("properties").and_then(|p| p.as_object());

        if let (Some(ap), Some(bp)) = (a_props, b_props) {
            let mut merged_props = ap.clone();
            for (key, val) in bp {
                if let Some(existing) = merged_props.get(key) {
                    merged_props.insert(key.clone(), merge_schemas(existing.clone(), val.clone()));
                } else {
                    merged_props.insert(key.clone(), val.clone());
                }
            }

            // Required = intersection of both required arrays
            let a_req: Vec<String> = a
                .get("required")
                .and_then(|r| r.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            let b_req: Vec<String> = b
                .get("required")
                .and_then(|r| r.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            let required: Vec<String> = a_req
                .iter()
                .filter(|k| b_req.contains(k))
                .cloned()
                .collect();

            return serde_json::json!({
                "type": "object",
                "properties": merged_props,
                "required": required
            });
        }
    }

    a
}

/// List schemas
pub fn list(path: &str, status_filter: Option<SchemaStatus>) -> Result<()> {
    let schemas_path = Path::new(path);
    let state = InferenceState::load(schemas_path)?;

    if state.schemas.is_empty() {
        println!("No schemas found in {}", path);
        println!();
        println!("Generate schemas from traffic with:");
        println!("  mpl schemas generate");
        return Ok(());
    }

    println!("Schemas in {}:", path);
    println!();
    println!("{:<40} {:<10} {:<8}", "STYPE", "STATUS", "SAMPLES");
    println!("{}", "-".repeat(60));

    for (stype, info) in &state.schemas {
        let show = match status_filter {
            None | Some(SchemaStatus::All) => true,
            Some(SchemaStatus::Active) => info.status == "active",
            Some(SchemaStatus::Pending) => info.status == "pending",
        };

        if show {
            println!(
                "{:<40} {:<10} {:<8}",
                stype, info.status, info.sample_count
            );
        }
    }

    Ok(())
}

/// Approve schemas
pub fn approve(path: &str, stype: Option<&str>, all: bool) -> Result<()> {
    let schemas_path = Path::new(path);
    let mut state = InferenceState::load(schemas_path)?;

    let mut approved = 0;

    if all {
        for (_, info) in state.schemas.iter_mut() {
            if info.status == "pending" {
                info.status = "active".to_string();
                info.updated_at = chrono::Utc::now().to_rfc3339();
                approved += 1;
            }
        }
    } else if let Some(target) = stype {
        if let Some(info) = state.schemas.get_mut(target) {
            if info.status == "pending" {
                info.status = "active".to_string();
                info.updated_at = chrono::Utc::now().to_rfc3339();
                approved += 1;
            } else {
                println!("Schema {} is already {}", target, info.status);
            }
        } else {
            anyhow::bail!("Schema not found: {}", target);
        }
    } else {
        anyhow::bail!("Specify an SType to approve or use --all");
    }

    state.save(schemas_path)?;
    println!("Approved {} schema(s)", approved);

    Ok(())
}

/// Check if a schema already exists in the registry
fn schema_exists_in_registry(output_path: &Path, stype: &str) -> Option<String> {
    let parts: Vec<&str> = stype.split('.').collect();
    if parts.len() < 4 {
        return None;
    }

    let version = parts.last().unwrap();
    let name = parts[parts.len() - 2];
    let namespace_parts = &parts[..parts.len() - 2];

    let mut dir_path = output_path.join("stypes");
    for part in namespace_parts {
        dir_path = dir_path.join(part);
    }
    dir_path = dir_path.join(name).join(version);

    let schema_file = dir_path.join("schema.json");
    if schema_file.exists() {
        // Read and compute hash of existing schema
        if let Ok(content) = fs::read_to_string(&schema_file) {
            if let Ok(schema) = serde_json::from_str::<serde_json::Value>(&content) {
                return Some(compute_schema_hash(&schema));
            }
        }
    }
    None
}

/// Find next available version for a schema
fn find_next_version(output_path: &Path, stype: &str) -> String {
    let parts: Vec<&str> = stype.split('.').collect();
    if parts.len() < 4 {
        return "v1".to_string();
    }

    let name = parts[parts.len() - 2];
    let namespace_parts = &parts[..parts.len() - 2];

    let mut type_dir = output_path.join("stypes");
    for part in namespace_parts {
        type_dir = type_dir.join(part);
    }
    type_dir = type_dir.join(name);

    // Find all existing versions
    let mut max_version = 0;
    if let Ok(entries) = fs::read_dir(&type_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with('v') {
                    if let Ok(num) = name[1..].parse::<u32>() {
                        max_version = max_version.max(num);
                    }
                }
            }
        }
    }

    format!("v{}", max_version + 1)
}

/// Export result for a single schema
#[derive(Debug)]
enum ExportResult {
    Exported,
    Skipped(&'static str),
    VersionBumped(String),
}

/// Export schemas to registry format
pub fn export(path: &str, output: &str, mode: ExportMode) -> Result<()> {
    let schemas_path = Path::new(path);
    let output_path = Path::new(output);
    let mut state = InferenceState::load(schemas_path)?;

    let active_stypes: Vec<String> = state
        .schemas
        .iter()
        .filter(|(_, info)| info.status == "active")
        .map(|(stype, _)| stype.clone())
        .collect();

    if active_stypes.is_empty() {
        println!("No active schemas to export.");
        println!("Approve schemas first with: mpl schemas approve --all");
        return Ok(());
    }

    // Create registry structure
    let stypes_path = output_path.join("stypes");
    fs::create_dir_all(&stypes_path)?;

    let mut exported = 0;
    let mut skipped = 0;
    let mut bumped = 0;

    for stype in active_stypes {
        let info = state.schemas.get(&stype).unwrap();
        let current_hash = info.schema_hash.clone().unwrap_or_else(|| compute_schema_hash(&info.schema));

        // Check if schema already exists in target registry
        let existing_hash = schema_exists_in_registry(output_path, &stype);

        let result = match mode {
            ExportMode::New => {
                if existing_hash.is_some() {
                    ExportResult::Skipped("already exists")
                } else {
                    export_schema(output_path, &stype, info)?;
                    ExportResult::Exported
                }
            }
            ExportMode::Delta => {
                match &existing_hash {
                    None => {
                        // New schema - export it
                        export_schema(output_path, &stype, info)?;
                        ExportResult::Exported
                    }
                    Some(existing) if existing != &current_hash => {
                        // Schema changed - export it
                        export_schema(output_path, &stype, info)?;
                        ExportResult::Exported
                    }
                    Some(_) => {
                        // Schema unchanged - skip
                        ExportResult::Skipped("unchanged")
                    }
                }
            }
            ExportMode::Full => {
                export_schema(output_path, &stype, info)?;
                ExportResult::Exported
            }
            ExportMode::BumpVersion => {
                match &existing_hash {
                    None => {
                        export_schema(output_path, &stype, info)?;
                        ExportResult::Exported
                    }
                    Some(existing) if existing != &current_hash => {
                        // Schema changed - create new version
                        let new_version = find_next_version(output_path, &stype);
                        let new_stype = bump_stype_version(&stype, &new_version);
                        export_schema(output_path, &new_stype, info)?;
                        ExportResult::VersionBumped(new_stype)
                    }
                    Some(_) => {
                        ExportResult::Skipped("unchanged")
                    }
                }
            }
        };

        match &result {
            ExportResult::Exported => {
                println!("  Exported: {}", stype);
                exported += 1;
                // Update export tracking in state
                if let Some(info) = state.schemas.get_mut(&stype) {
                    info.exported_at = Some(chrono::Utc::now().to_rfc3339());
                    info.exported_hash = Some(current_hash.clone());
                }
            }
            ExportResult::Skipped(reason) => {
                println!("  Skipped: {} ({})", stype, reason);
                skipped += 1;
            }
            ExportResult::VersionBumped(new_stype) => {
                println!("  Bumped: {} -> {}", stype, new_stype);
                bumped += 1;
                if let Some(info) = state.schemas.get_mut(&stype) {
                    info.exported_at = Some(chrono::Utc::now().to_rfc3339());
                    info.exported_hash = Some(current_hash.clone());
                }
            }
        }
    }

    // Save updated state with export tracking
    state.save(schemas_path)?;

    println!();
    println!("Export complete ({:?} mode):", mode);
    println!("  Exported: {}", exported);
    if skipped > 0 {
        println!("  Skipped: {}", skipped);
    }
    if bumped > 0 {
        println!("  Version bumped: {}", bumped);
    }
    println!("  Registry: {}", output);

    Ok(())
}

/// Export a single schema to the registry
fn export_schema(output_path: &Path, stype: &str, info: &SchemaInfo) -> Result<()> {
    let parts: Vec<&str> = stype.split('.').collect();
    if parts.len() < 4 {
        anyhow::bail!("Invalid stype format: {}", stype);
    }

    let version = parts.last().unwrap();
    let name = parts[parts.len() - 2];
    let namespace_parts = &parts[..parts.len() - 2];

    // Create directory structure
    let mut dir_path = output_path.join("stypes");
    for part in namespace_parts {
        dir_path = dir_path.join(part);
    }
    dir_path = dir_path.join(name).join(version);
    fs::create_dir_all(&dir_path)?;

    // Write schema.json
    let schema_file = dir_path.join("schema.json");
    let content = serde_json::to_string_pretty(&info.schema)?;
    fs::write(&schema_file, content)?;

    // Write manifest.json
    let manifest = serde_json::json!({
        "stype": stype,
        "version": version,
        "created": info.created_at,
        "updated": info.updated_at,
        "sample_count": info.sample_count,
        "description": format!("Auto-generated from {} samples", info.sample_count)
    });
    let manifest_file = dir_path.join("manifest.json");
    fs::write(manifest_file, serde_json::to_string_pretty(&manifest)?)?;

    Ok(())
}

/// Bump the version in an stype string
fn bump_stype_version(stype: &str, new_version: &str) -> String {
    let parts: Vec<&str> = stype.split('.').collect();
    if parts.len() < 4 {
        return stype.to_string();
    }

    let mut new_parts: Vec<&str> = parts[..parts.len() - 1].to_vec();
    new_parts.push(new_version);
    new_parts.join(".")
}

/// Show schema details
pub fn show(path: &str, stype: &str) -> Result<()> {
    let schemas_path = Path::new(path);
    let state = InferenceState::load(schemas_path)?;

    let info = state
        .schemas
        .get(stype)
        .with_context(|| format!("Schema not found: {}", stype))?;

    println!("SType: {}", stype);
    println!("Status: {}", info.status);
    println!("Samples: {}", info.sample_count);
    println!("Created: {}", info.created_at);
    println!("Updated: {}", info.updated_at);
    if let Some(hash) = &info.schema_hash {
        println!("Hash: {}...", &hash[..16]);
    }
    if let Some(exported_at) = &info.exported_at {
        println!("Exported: {}", exported_at);
    }
    if info.exported_hash != info.schema_hash {
        println!("  (schema changed since last export)");
    }
    println!();
    println!("Schema:");
    println!("{}", serde_json::to_string_pretty(&info.schema)?);

    Ok(())
}
