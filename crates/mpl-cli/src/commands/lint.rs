//! Lint registry for errors

use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub fn run(path: &str) -> Result<()> {
    println!("{} Linting registry at {}", "→".blue(), path);

    let base_path = Path::new(path);
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut schema_count = 0;
    let mut tool_count = 0;
    let mut profile_count = 0;

    // Check stypes directory
    let stypes_path = base_path.join("stypes");
    if stypes_path.exists() {
        for entry in WalkDir::new(&stypes_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |e| e == "json") {
                if path.file_name().map_or(false, |n| n == "schema.json") {
                    schema_count += 1;
                    // Validate schema
                    match fs::read_to_string(path) {
                        Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                            Ok(schema) => {
                                // Check for required fields
                                if !schema.get("$schema").is_some() {
                                    warnings.push(format!(
                                        "{}: missing $schema field",
                                        path.display()
                                    ));
                                }
                                if !schema.get("type").is_some() {
                                    errors.push(format!("{}: missing type field", path.display()));
                                }
                            }
                            Err(e) => {
                                errors.push(format!("{}: invalid JSON: {}", path.display(), e));
                            }
                        },
                        Err(e) => {
                            errors.push(format!("{}: could not read: {}", path.display(), e));
                        }
                    }
                }
            }
        }
    }

    // Check tools directory
    let tools_path = base_path.join("tools");
    if tools_path.exists() {
        for entry in fs::read_dir(&tools_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "json") {
                tool_count += 1;
                match fs::read_to_string(&path) {
                    Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                        Ok(tool) => {
                            if !tool.get("id").is_some() {
                                errors.push(format!("{}: missing id field", path.display()));
                            }
                            if !tool.get("args_stype").is_some() {
                                warnings.push(format!("{}: missing args_stype", path.display()));
                            }
                        }
                        Err(e) => {
                            errors.push(format!("{}: invalid JSON: {}", path.display(), e));
                        }
                    },
                    Err(e) => {
                        errors.push(format!("{}: could not read: {}", path.display(), e));
                    }
                }
            }
        }
    }

    // Check profiles directory
    let profiles_path = base_path.join("profiles");
    if profiles_path.exists() {
        for entry in fs::read_dir(&profiles_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "json") {
                profile_count += 1;
                match fs::read_to_string(&path) {
                    Ok(content) => {
                        if serde_json::from_str::<serde_json::Value>(&content).is_err() {
                            errors.push(format!("{}: invalid JSON", path.display()));
                        }
                    }
                    Err(e) => {
                        errors.push(format!("{}: could not read: {}", path.display(), e));
                    }
                }
            }
        }
    }

    // Print results
    println!("\n{}", "Summary:".yellow().bold());
    println!("  Schemas:  {}", schema_count);
    println!("  Tools:    {}", tool_count);
    println!("  Profiles: {}", profile_count);

    if !warnings.is_empty() {
        println!("\n{} ({}):", "Warnings".yellow().bold(), warnings.len());
        for warning in &warnings {
            println!("  {} {}", "⚠".yellow(), warning);
        }
    }

    if !errors.is_empty() {
        println!("\n{} ({}):", "Errors".red().bold(), errors.len());
        for error in &errors {
            println!("  {} {}", "✗".red(), error);
        }
        anyhow::bail!("Lint failed with {} errors", errors.len());
    }

    println!("\n{} Lint passed!", "✓".green().bold());

    Ok(())
}
