//! Conformance test runner
//!
//! Runs validation tests against all STypes in the registry to verify
//! schema correctness and example payload validity.

use anyhow::{Context, Result};
use mpl_core::validation::SchemaValidator;
use serde_json::Value;
use std::fs;
use std::path::Path;
use tracing::{error, info, warn};

/// Conformance test result
#[derive(Default)]
struct ConformanceResults {
    stypes_tested: usize,
    schemas_valid: usize,
    schemas_invalid: usize,
    examples_tested: usize,
    examples_passed: usize,
    examples_failed: usize,
    negative_tested: usize,
    negative_passed: usize,
    negative_failed: usize,
}

impl ConformanceResults {
    fn print_summary(&self) {
        println!("\n╔══════════════════════════════════════════╗");
        println!("║         CONFORMANCE TEST RESULTS         ║");
        println!("╠══════════════════════════════════════════╣");
        println!(
            "║  STypes tested:        {:>6}            ║",
            self.stypes_tested
        );
        println!(
            "║    ✓ Valid schemas:    {:>6}            ║",
            self.schemas_valid
        );
        println!(
            "║    ✗ Invalid schemas:  {:>6}            ║",
            self.schemas_invalid
        );
        println!("╠══════════════════════════════════════════╣");
        println!(
            "║  Positive examples:    {:>6}            ║",
            self.examples_tested
        );
        println!(
            "║    ✓ Passed:           {:>6}            ║",
            self.examples_passed
        );
        println!(
            "║    ✗ Failed:           {:>6}            ║",
            self.examples_failed
        );
        println!("╠══════════════════════════════════════════╣");
        println!(
            "║  Negative examples:    {:>6}            ║",
            self.negative_tested
        );
        println!(
            "║    ✓ Correctly rejected:{:>5}            ║",
            self.negative_passed
        );
        println!(
            "║    ✗ Incorrectly passed:{:>5}            ║",
            self.negative_failed
        );
        println!("╚══════════════════════════════════════════╝");

        let total_failures = self.schemas_invalid + self.examples_failed + self.negative_failed;
        if total_failures == 0 {
            println!("\n✓ All conformance tests passed!");
        } else {
            println!("\n✗ {} failures detected", total_failures);
        }
    }
}

/// Run conformance tests on a registry
pub fn run(registry_path: &str, stype_filter: Option<&str>, verbose: bool) -> Result<()> {
    let registry = Path::new(registry_path);
    let stypes_dir = registry.join("stypes");

    if !stypes_dir.exists() {
        anyhow::bail!(
            "Registry stypes directory not found: {}",
            stypes_dir.display()
        );
    }

    info!("Running conformance tests on: {}", registry_path);
    let mut results = ConformanceResults::default();
    let mut validator = SchemaValidator::new();

    // Walk the registry and test each SType
    walk_stypes(
        &stypes_dir,
        Vec::new(),
        &mut validator,
        &mut results,
        stype_filter,
        verbose,
    )?;

    results.print_summary();

    if results.schemas_invalid + results.examples_failed + results.negative_failed > 0 {
        anyhow::bail!("Conformance tests failed");
    }

    Ok(())
}

fn walk_stypes(
    path: &Path,
    parts: Vec<String>,
    validator: &mut SchemaValidator,
    results: &mut ConformanceResults,
    filter: Option<&str>,
    verbose: bool,
) -> Result<()> {
    if !path.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if entry_path.is_dir() {
            let mut new_parts = parts.clone();
            new_parts.push(name);
            walk_stypes(&entry_path, new_parts, validator, results, filter, verbose)?;
        } else if name == "schema.json" && parts.len() >= 3 {
            // Build SType name from path parts
            let stype = build_stype_name(&parts);

            // Apply filter if specified
            if let Some(f) = filter {
                if !stype.contains(f) {
                    continue;
                }
            }

            test_stype(&entry_path, &stype, validator, results, verbose)?;
        }
    }

    Ok(())
}

fn build_stype_name(parts: &[String]) -> String {
    // parts = [namespace..., Name, vN]
    if parts.len() < 3 {
        return parts.join(".");
    }

    let version_str = &parts[parts.len() - 1];
    let name = &parts[parts.len() - 2];
    let namespace = parts[..parts.len() - 2].join(".");

    format!("{}.{}.{}", namespace, name, version_str)
}

fn test_stype(
    schema_path: &Path,
    stype: &str,
    validator: &mut SchemaValidator,
    results: &mut ConformanceResults,
    verbose: bool,
) -> Result<()> {
    results.stypes_tested += 1;

    if verbose {
        println!("\nTesting: {}", stype);
    }

    // 1. Load and validate schema
    let schema_content = fs::read_to_string(schema_path)
        .with_context(|| format!("Failed to read schema: {}", schema_path.display()))?;

    // Check schema is valid JSON
    let _schema_json: Value = serde_json::from_str(&schema_content)
        .with_context(|| format!("Invalid JSON in schema: {}", schema_path.display()))?;

    // Register schema
    match validator.register_json(stype, &schema_content) {
        Ok(_) => {
            results.schemas_valid += 1;
            if verbose {
                println!("  ✓ Schema valid");
            }
        }
        Err(e) => {
            results.schemas_invalid += 1;
            error!("  ✗ Schema invalid: {}", e);
            return Ok(()); // Skip examples if schema is invalid
        }
    }

    // 2. Test positive examples (should pass validation)
    if let Some(parent) = schema_path.parent() {
        let examples_dir = parent.join("examples");
        if examples_dir.exists() {
            test_examples(&examples_dir, stype, validator, results, verbose, true)?;
        }

        // 3. Test negative examples (should fail validation)
        let negative_dir = parent.join("negative");
        if negative_dir.exists() {
            test_examples(&negative_dir, stype, validator, results, verbose, false)?;
        }
    }

    Ok(())
}

fn test_examples(
    dir: &Path,
    stype: &str,
    validator: &SchemaValidator,
    results: &mut ConformanceResults,
    verbose: bool,
    expect_valid: bool,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().is_some_and(|ext| ext == "json") {
            let content = fs::read_to_string(&path)?;
            let payload: Value = match serde_json::from_str(&content) {
                Ok(v) => v,
                Err(e) => {
                    warn!("  ⚠ Invalid JSON in example {}: {}", path.display(), e);
                    continue;
                }
            };

            let validation = validator.validate(stype, &payload);
            let file_name = path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| path.display().to_string());

            if expect_valid {
                results.examples_tested += 1;
                match validation {
                    Ok(v) if v.valid => {
                        results.examples_passed += 1;
                        if verbose {
                            println!("  ✓ Example passed: {}", file_name);
                        }
                    }
                    Ok(v) => {
                        results.examples_failed += 1;
                        error!("  ✗ Example failed: {} (expected valid)", file_name);
                        for err in v.errors {
                            error!("      - {} at {}", err.message, err.path);
                        }
                    }
                    Err(e) => {
                        results.examples_failed += 1;
                        error!("  ✗ Example failed: {} ({})", file_name, e);
                    }
                }
            } else {
                results.negative_tested += 1;
                let is_valid = validation.map(|v| v.valid).unwrap_or(false);
                if !is_valid {
                    results.negative_passed += 1;
                    if verbose {
                        println!("  ✓ Negative correctly rejected: {}", file_name);
                    }
                } else {
                    results.negative_failed += 1;
                    error!(
                        "  ✗ Negative incorrectly passed: {} (expected invalid)",
                        file_name
                    );
                }
            }
        }
    }

    Ok(())
}
