//! Validate a payload against an SType schema

use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::Path;

use mpl_core::stype::SType;
use mpl_core::validation::SchemaValidator;

/// Load payload from JSON string or file path
fn load_payload(input: &str) -> Result<serde_json::Value> {
    // Try to parse as JSON first
    if let Ok(payload) = serde_json::from_str(input) {
        return Ok(payload);
    }

    // Otherwise treat as file path
    let content = fs::read_to_string(input)?;
    Ok(serde_json::from_str(&content)?)
}

/// Find schema in registry for an SType
fn find_schema_in_registry(stype: &SType, registry_path: &str) -> Result<String> {
    // Build path: registry/stypes/namespace/domain/Name/vN/schema.json
    let schema_path = Path::new(registry_path)
        .join("stypes")
        .join(&stype.namespace)
        .join(&stype.domain)
        .join(&stype.name)
        .join(format!("v{}", stype.major_version))
        .join("schema.json");

    if schema_path.exists() {
        Ok(fs::read_to_string(schema_path)?)
    } else {
        anyhow::bail!(
            "Schema not found in registry at: {}",
            schema_path.display()
        );
    }
}

pub fn run(stype: &str, payload_input: &str, schema_path: Option<&str>, registry_path: &str) -> Result<()> {
    println!("{} Validating payload against {}", "→".blue(), stype.green());

    // Parse SType
    let parsed_stype = SType::parse(stype)?;

    // Load payload
    let payload = load_payload(payload_input)?;

    // Load schema from path or registry
    let schema_str = match schema_path {
        Some(path) => fs::read_to_string(path)?,
        None => find_schema_in_registry(&parsed_stype, registry_path)?,
    };
    let schema: serde_json::Value = serde_json::from_str(&schema_str)?;

    // Create validator and register schema
    let mut validator = SchemaValidator::new();
    validator.register(stype, schema)?;

    // Validate
    let result = validator.validate(stype, &payload)?;

    if result.valid {
        println!("{} Validation passed!", "✓".green().bold());
        println!("  Schema Fidelity: {}", "1.0".green());
    } else {
        println!("{} Validation failed!", "✗".red().bold());
        println!("  Schema Fidelity: {}", "0.0".red());
        println!("\nErrors:");
        for error in &result.errors {
            println!("  {} {}: {}", "•".red(), error.path.yellow(), error.message);
        }
        // Return error code for CI/scripts
        std::process::exit(1);
    }

    Ok(())
}
