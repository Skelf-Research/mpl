//! Initialize a new registry namespace

use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::Path;

pub fn run(namespace: &str, output: &str) -> Result<()> {
    println!("{} Initializing namespace: {}", "→".blue(), namespace.green());

    let base_path = Path::new(output);

    // Create directory structure
    let namespace_path = namespace.replace('.', "/");

    let dirs = [
        format!("stypes/{}", namespace_path),
        "tools".to_string(),
        "profiles".to_string(),
        "policies".to_string(),
        "adapters".to_string(),
    ];

    for dir in &dirs {
        let full_path = base_path.join(dir);
        fs::create_dir_all(&full_path)?;
        println!("  {} Created {}", "✓".green(), dir);
    }

    // Create CODEOWNERS file
    let codeowners_content = format!(
        "# MPL Registry CODEOWNERS\n\
         # See: https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners\n\n\
         /stypes/{}/ @{}-maintainers\n",
        namespace_path,
        namespace.replace('.', "-")
    );
    fs::write(base_path.join("CODEOWNERS"), codeowners_content)?;
    println!("  {} Created CODEOWNERS", "✓".green());

    // Create README
    let readme_content = format!(
        "# {} MPL Registry\n\n\
         This directory contains MPL artifacts for the `{}` namespace.\n\n\
         ## Structure\n\n\
         ```\n\
         stypes/     # Semantic Type schemas\n\
         tools/      # Tool descriptors\n\
         profiles/   # QoM profiles\n\
         policies/   # Policy manifests\n\
         adapters/   # Version adapters\n\
         ```\n\n\
         ## Adding a new SType\n\n\
         ```bash\n\
         mpl add-stype {}.MyType.v1 schema.json --examples examples/\n\
         ```\n\n\
         ## Validation\n\n\
         ```bash\n\
         mpl lint\n\
         ```\n",
        namespace, namespace, namespace
    );
    fs::write(base_path.join("README.md"), readme_content)?;
    println!("  {} Created README.md", "✓".green());

    // Create a sample QoM profile
    let basic_profile = serde_json::json!({
        "name": "qom-basic",
        "metrics": {
            "schema_fidelity": {"min": 1.0}
        },
        "description": "Basic validation: Schema Fidelity only"
    });
    fs::write(
        base_path.join("profiles/qom-basic.json"),
        serde_json::to_string_pretty(&basic_profile)?,
    )?;
    println!("  {} Created profiles/qom-basic.json", "✓".green());

    println!(
        "\n{} Namespace {} initialized successfully!",
        "✓".green().bold(),
        namespace.green()
    );
    println!("\nNext steps:");
    println!("  1. Add STypes: mpl add-stype {}.MyType.v1 schema.json", namespace);
    println!("  2. Lint: mpl lint");
    println!("  3. Commit and push to registry");

    Ok(())
}
