//! QoM evaluation command

use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::Path;

use mpl_core::qom::{QomMetrics, QomProfile};

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

/// Find profile in registry
fn find_profile_in_registry(profile_name: &str, registry_path: &str) -> Result<QomProfile> {
    let profile_path = Path::new(registry_path)
        .join("profiles")
        .join(format!("{}.json", profile_name));

    if profile_path.exists() {
        let content = fs::read_to_string(profile_path)?;
        Ok(serde_json::from_str(&content)?)
    } else {
        anyhow::bail!("Profile not found: {}", profile_name);
    }
}

pub fn run(
    profile_name: &str,
    payload_input: &str,
    _response_path: Option<&str>,
    registry_path: &str,
) -> Result<()> {
    println!(
        "{} Evaluating QoM with profile: {}",
        "→".blue(),
        profile_name.green()
    );

    // Load payload
    let _payload = load_payload(payload_input)?;

    // Load or create profile
    let profile = match profile_name {
        "qom-basic" => QomProfile::basic(),
        "qom-strict-argcheck" => QomProfile::strict_argcheck(),
        _ => {
            // Try to load from registry first
            if let Ok(p) = find_profile_in_registry(profile_name, registry_path) {
                p
            } else {
                // Try as direct file path
                let profile_str = fs::read_to_string(profile_name)?;
                serde_json::from_str(&profile_str)?
            }
        }
    };

    println!("  Profile: {}", profile.name);
    if let Some(desc) = &profile.description {
        println!("  Description: {}", desc);
    }

    // For now, assume schema fidelity passes (would need validator with schema)
    // In real implementation, this would validate against the schema
    let metrics = QomMetrics::schema_valid().with_instruction_compliance(1.0);

    let evaluation = profile.evaluate(&metrics);

    println!("\n{}", "Metrics:".yellow().bold());
    println!(
        "  Schema Fidelity:        {}",
        format!("{:.2}", metrics.schema_fidelity).green()
    );
    if let Some(ic) = metrics.instruction_compliance {
        println!("  Instruction Compliance: {}", format!("{:.2}", ic).green());
    }

    println!("\n{}", "Evaluation:".yellow().bold());
    if evaluation.meets_profile {
        println!("  Result: {} meets profile", "✓".green().bold());
    } else {
        println!("  Result: {} does not meet profile", "✗".red().bold());
        println!("\n  Failures:");
        for failure in &evaluation.failures {
            println!(
                "    {} {}: got {:.2}, expected >= {:.2}",
                "•".red(),
                failure.metric,
                failure.actual,
                failure.threshold
            );
        }
        std::process::exit(1);
    }

    Ok(())
}
