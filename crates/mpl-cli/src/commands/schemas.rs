//! Schema management commands
//!
//! Commands for schema inference, approval, and management.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::SchemaStatus;

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
                let info = SchemaInfo {
                    stype: stype.clone(),
                    status: "pending".to_string(),
                    sample_count: samples.len(),
                    created_at: chrono::Utc::now().to_rfc3339(),
                    updated_at: chrono::Utc::now().to_rfc3339(),
                    schema,
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

/// Export schemas to registry format
pub fn export(path: &str, output: &str) -> Result<()> {
    let schemas_path = Path::new(path);
    let output_path = Path::new(output);
    let state = InferenceState::load(schemas_path)?;

    let active_schemas: Vec<_> = state
        .schemas
        .iter()
        .filter(|(_, info)| info.status == "active")
        .collect();

    if active_schemas.is_empty() {
        println!("No active schemas to export.");
        println!("Approve schemas first with: mpl schemas approve --all");
        return Ok(());
    }

    // Create registry structure
    let stypes_path = output_path.join("stypes");
    fs::create_dir_all(&stypes_path)?;

    for (stype, info) in active_schemas {
        // Parse stype: namespace.domain.Name.vN
        let parts: Vec<&str> = stype.split('.').collect();
        if parts.len() < 4 {
            println!("Skipping invalid stype: {}", stype);
            continue;
        }

        let version = parts.last().unwrap();
        let name = parts[parts.len() - 2];
        let namespace_parts = &parts[..parts.len() - 2];

        // Create directory structure
        let mut dir_path = stypes_path.clone();
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
            "description": format!("Auto-generated from {} samples", info.sample_count)
        });
        let manifest_file = dir_path.join("manifest.json");
        fs::write(manifest_file, serde_json::to_string_pretty(&manifest)?)?;

        println!("Exported: {}", stype);
    }

    println!();
    println!("Registry exported to: {}", output);

    Ok(())
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
    println!();
    println!("Schema:");
    println!("{}", serde_json::to_string_pretty(&info.schema)?);

    Ok(())
}
