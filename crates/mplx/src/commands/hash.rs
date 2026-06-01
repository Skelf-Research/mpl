//! Compute semantic hash of a payload

use anyhow::Result;
use colored::Colorize;
use std::fs;

use mpl_core::hash::{canonicalize, semantic_hash};

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

pub fn run(payload_input: &str) -> Result<()> {
    println!("{} Computing semantic hash", "→".blue());

    // Load payload
    let payload = load_payload(payload_input)?;

    // Canonicalize
    let canonical = canonicalize(&payload)?;

    // Compute hash
    let hash = semantic_hash(&payload)?;

    println!("\n{}", "Canonical form:".yellow());
    println!("{}", canonical);

    println!("\n{}", "Semantic hash:".yellow());
    println!("{}", hash.green().bold());

    Ok(())
}
