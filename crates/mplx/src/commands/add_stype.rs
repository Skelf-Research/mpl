//! Add a new SType to the registry

use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::Path;

use mpl_core::stype::SType;

pub fn run(stype_str: &str, schema_path: &str, examples: &[String]) -> Result<()> {
    println!("{} Adding SType: {}", "→".blue(), stype_str.green());

    // Parse SType
    let stype = SType::parse(stype_str)?;

    // Create directory structure
    let dir_path = format!(
        "stypes/{}/{}/{}/v{}",
        stype.namespace.replace('.', "/"),
        stype.domain,
        stype.name,
        stype.major_version
    );
    fs::create_dir_all(&dir_path)?;
    println!("  {} Created {}", "✓".green(), dir_path);

    // Copy schema
    let schema_content = fs::read_to_string(schema_path)?;
    let mut schema: serde_json::Value = serde_json::from_str(&schema_content)?;

    // Add $id if not present
    if schema.get("$id").is_none() {
        schema["$id"] = serde_json::Value::String(stype.urn());
    }

    fs::write(
        format!("{}/schema.json", dir_path),
        serde_json::to_string_pretty(&schema)?,
    )?;
    println!("  {} Created schema.json", "✓".green());

    // Copy examples
    if !examples.is_empty() {
        let examples_dir = format!("{}/examples", dir_path);
        fs::create_dir_all(&examples_dir)?;

        for example_path in examples {
            let path = Path::new(example_path);
            let filename = path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| {
                    // Fallback to using the full path as filename if extraction fails
                    example_path.replace(['/', '\\'], "_")
                });
            fs::copy(example_path, format!("{}/{}", examples_dir, filename))?;
            println!("  {} Copied example: {}", "✓".green(), filename);
        }
    }

    // Create README
    let readme = format!(
        "# {}\n\n\
         SType: `{}`\n\n\
         ## Description\n\n\
         TODO: Add description\n\n\
         ## Schema\n\n\
         See `schema.json`\n\n\
         ## Examples\n\n\
         See `examples/` directory\n",
        stype.name, stype_str
    );
    fs::write(format!("{}/README.md", dir_path), readme)?;
    println!("  {} Created README.md", "✓".green());

    // Create CHANGELOG
    let changelog = format!(
        "# Changelog\n\n\
         ## v{} - {}\n\n\
         - Initial release\n",
        stype.major_version,
        chrono::Utc::now().format("%Y-%m-%d")
    );
    fs::write(format!("{}/CHANGELOG.md", dir_path), changelog)?;
    println!("  {} Created CHANGELOG.md", "✓".green());

    println!(
        "\n{} SType {} added successfully!",
        "✓".green().bold(),
        stype_str.green()
    );

    Ok(())
}
