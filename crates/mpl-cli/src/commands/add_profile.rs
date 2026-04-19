//! Add a QoM profile

use anyhow::Result;
use colored::Colorize;
use std::fs;

pub fn run(name: &str, profile_path: &str) -> Result<()> {
    println!("{} Adding QoM profile: {}", "→".blue(), name.green());

    // Load and validate profile
    let content = fs::read_to_string(profile_path)?;
    let mut profile: serde_json::Value = serde_json::from_str(&content)?;

    // Ensure name matches
    profile["name"] = serde_json::Value::String(name.to_string());

    // Ensure profiles directory exists
    fs::create_dir_all("profiles")?;

    // Write profile
    let filename = format!("profiles/{}.json", name);
    fs::write(&filename, serde_json::to_string_pretty(&profile)?)?;

    println!("  {} Created {}", "✓".green(), filename);
    println!(
        "\n{} Profile {} added successfully!",
        "✓".green().bold(),
        name.green()
    );

    Ok(())
}
