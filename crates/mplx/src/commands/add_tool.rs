//! Add a new tool descriptor

use anyhow::Result;
use colored::Colorize;
use std::fs;

pub fn run(
    tool_id: &str,
    args_stype: &str,
    returns_stype: &str,
    profile: Option<String>,
    policy: Option<String>,
) -> Result<()> {
    println!("{} Adding tool: {}", "→".blue(), tool_id.green());

    // Create tool descriptor
    let mut tool = serde_json::json!({
        "id": tool_id,
        "name": tool_id.split('.').take(2).collect::<Vec<_>>().join("."),
        "args_stype": args_stype,
        "returns_stype": returns_stype,
        "features": [],
    });

    if let Some(p) = profile {
        tool["profiles"] = serde_json::json!([p]);
    }

    if let Some(p) = policy {
        tool["policies"] = serde_json::json!([p]);
    }

    // Ensure tools directory exists
    fs::create_dir_all("tools")?;

    // Write tool descriptor
    let filename = format!("tools/tool.{}.json", tool_id);
    fs::write(&filename, serde_json::to_string_pretty(&tool)?)?;

    println!("  {} Created {}", "✓".green(), filename);
    println!(
        "\n{} Tool {} added successfully!",
        "✓".green().bold(),
        tool_id.green()
    );

    Ok(())
}
