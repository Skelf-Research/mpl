//! MPL CLI
//!
//! Unified command-line interface for MPL:
//! - `mpl proxy <upstream>` - Start proxy with zero config
//! - `mpl schemas` - Schema management and inference
//! - `mpl ui` - Launch web dashboard
//! - Registry management (init, lint, validate, etc.)

mod commands;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(name = "mpl")]
#[command(about = "MPL - Meaning Protocol Layer")]
#[command(version)]
#[command(after_help = "QUICK START:
    mpl proxy http://mcp-server:8080    Start proxy (zero config)
    mpl ui                               Open web dashboard
    mpl schemas generate                 Generate schemas from traffic
")]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Path to data directory
    #[arg(long, global = true, default_value = "~/.mpl")]
    data_dir: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the MPL proxy (zero-config)
    #[command(alias = "p")]
    Proxy {
        /// Upstream server URL (e.g., http://mcp-server:8080)
        upstream: String,

        /// Listen address
        #[arg(short, long, default_value = "0.0.0.0:9443")]
        listen: String,

        /// Mode: development (log only) or production (enforce)
        #[arg(short, long, default_value = "development")]
        mode: Mode,

        /// Enable schema learning from traffic
        #[arg(long, default_value = "true")]
        learn: bool,

        /// Path to schemas directory
        #[arg(long)]
        schemas: Option<String>,

        /// Metrics port (0 to disable)
        #[arg(long, default_value = "9100")]
        metrics_port: u16,

        /// Enable web UI
        #[arg(long, default_value = "true")]
        ui: bool,

        /// Web UI port
        #[arg(long, default_value = "9080")]
        ui_port: u16,
    },

    /// Schema management and inference
    Schemas {
        #[command(subcommand)]
        command: SchemasCommands,
    },

    /// Launch web dashboard
    #[command(alias = "dashboard")]
    Ui {
        /// Port for web UI
        #[arg(short, long, default_value = "9080")]
        port: u16,

        /// Open browser automatically
        #[arg(long, default_value = "true")]
        open: bool,
    },

    // ===== Registry Commands (existing) =====
    /// Initialize a new registry namespace
    Init {
        /// Namespace to initialize (e.g., "org.mycompany")
        namespace: String,

        /// Output directory
        #[arg(short, long, default_value = ".")]
        output: String,
    },

    /// Add a new SType to the registry
    AddStype {
        /// SType identifier (e.g., "org.calendar.Event.v1")
        stype: String,

        /// Path to schema file
        schema: String,

        /// Example files
        #[arg(long)]
        examples: Vec<String>,
    },

    /// Lint registry for errors
    Lint {
        /// Directory to lint
        #[arg(default_value = ".")]
        path: String,
    },

    /// Validate a payload against an SType schema
    Validate {
        /// SType identifier
        #[arg(long)]
        stype: String,

        /// Payload (JSON string or path to file)
        payload: String,

        /// Path to schema file (if not using registry)
        #[arg(long)]
        schema: Option<String>,

        /// Path to registry directory
        #[arg(short, long, default_value = "./registry")]
        registry: String,
    },

    /// Add a new tool descriptor
    AddTool {
        /// Tool identifier (e.g., "calendar.create.v1")
        tool_id: String,

        /// Arguments SType
        #[arg(long)]
        args_stype: String,

        /// Returns SType
        #[arg(long)]
        returns_stype: String,

        /// QoM profile
        #[arg(long)]
        profile: Option<String>,

        /// Policy reference
        #[arg(long)]
        policy: Option<String>,
    },

    /// Add a QoM profile
    AddProfile {
        /// Profile name
        name: String,

        /// Path to profile JSON file
        profile: String,
    },

    /// Compute semantic hash of a payload
    Hash {
        /// Payload (JSON string or path to file)
        payload: String,
    },

    /// Run QoM evaluation on a payload
    Qom {
        #[command(subcommand)]
        command: QomCommands,
    },

    /// Run conformance tests on the registry
    Conformance {
        /// Path to registry
        #[arg(short, long, default_value = "./registry")]
        registry: String,

        /// Filter STypes by pattern (e.g., "calendar" or "org.finance")
        #[arg(long)]
        filter: Option<String>,
    },
}

#[derive(Subcommand)]
enum SchemasCommands {
    /// Generate schemas from recorded traffic
    Generate {
        /// Minimum samples required for inference
        #[arg(long, default_value = "10")]
        min_samples: usize,

        /// Output directory for generated schemas
        #[arg(short, long, default_value = "./schemas")]
        output: String,
    },

    /// List all schemas (active and pending)
    List {
        /// Filter by status
        #[arg(long)]
        status: Option<SchemaStatus>,

        /// Path to schemas directory
        #[arg(short, long, default_value = "./schemas")]
        path: String,
    },

    /// Approve a pending schema
    Approve {
        /// SType to approve (or 'all' for all pending)
        stype: Option<String>,

        /// Approve all pending schemas
        #[arg(long)]
        all: bool,

        /// Path to schemas directory
        #[arg(short, long, default_value = "./schemas")]
        path: String,
    },

    /// Export schemas to registry format
    Export {
        /// Output directory
        #[arg(short, long, default_value = "./registry")]
        output: String,

        /// Path to schemas directory
        #[arg(short, long, default_value = "./schemas")]
        path: String,

        /// Export mode: new (skip existing), delta (export changes), full (overwrite all), bump (version changed)
        #[arg(short, long, default_value = "delta")]
        mode: ExportModeArg,
    },

    /// Show schema details
    Show {
        /// SType to show
        stype: String,

        /// Path to schemas directory
        #[arg(short, long, default_value = "./schemas")]
        path: String,
    },
}

#[derive(Subcommand)]
enum QomCommands {
    /// Evaluate a payload against a QoM profile
    Evaluate {
        /// QoM profile name or path
        #[arg(long)]
        profile: String,

        /// Request payload path
        #[arg(long)]
        payload: String,

        /// Response payload path (for response validation)
        #[arg(long)]
        response: Option<String>,

        /// Path to registry
        #[arg(short, long, default_value = "./registry")]
        registry: String,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Mode {
    /// Log violations but don't block requests
    Development,
    /// Enforce validation and block invalid requests
    Production,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum SchemaStatus {
    /// Active schemas being enforced
    Active,
    /// Pending schemas awaiting approval
    Pending,
    /// All schemas
    All,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ExportModeArg {
    /// Only export new schemas (skip existing)
    New,
    /// Export new and changed schemas (default)
    Delta,
    /// Overwrite all schemas
    Full,
    /// Bump version for changed schemas
    Bump,
}

impl From<ExportModeArg> for commands::schemas::ExportMode {
    fn from(arg: ExportModeArg) -> Self {
        match arg {
            ExportModeArg::New => commands::schemas::ExportMode::New,
            ExportModeArg::Delta => commands::schemas::ExportMode::Delta,
            ExportModeArg::Full => commands::schemas::ExportMode::Full,
            ExportModeArg::Bump => commands::schemas::ExportMode::BumpVersion,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Expand ~ in data_dir
    let data_dir = shellexpand::tilde(&cli.data_dir).to_string();

    // Set up logging
    let level = if cli.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };
    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    match cli.command {
        Commands::Proxy {
            upstream,
            listen,
            mode,
            learn,
            schemas,
            metrics_port,
            ui,
            ui_port,
        } => {
            commands::proxy::run(
                &upstream,
                &listen,
                mode,
                learn,
                schemas.as_deref(),
                metrics_port,
                ui,
                ui_port,
                &data_dir,
                cli.verbose,
            )
            .await?;
        }

        Commands::Schemas { command } => match command {
            SchemasCommands::Generate {
                min_samples,
                output,
            } => {
                commands::schemas::generate(&data_dir, min_samples, &output)?;
            }
            SchemasCommands::List { status, path } => {
                commands::schemas::list(&path, status)?;
            }
            SchemasCommands::Approve { stype, all, path } => {
                commands::schemas::approve(&path, stype.as_deref(), all)?;
            }
            SchemasCommands::Export { output, path, mode } => {
                commands::schemas::export(&path, &output, mode.into())?;
            }
            SchemasCommands::Show { stype, path } => {
                commands::schemas::show(&path, &stype)?;
            }
        },

        Commands::Ui { port, open } => {
            commands::ui::run(port, open, &data_dir).await?;
        }

        // Existing commands
        Commands::Init { namespace, output } => {
            commands::init::run(&namespace, &output)?;
        }
        Commands::AddStype {
            stype,
            schema,
            examples,
        } => {
            commands::add_stype::run(&stype, &schema, &examples)?;
        }
        Commands::Lint { path } => {
            commands::lint::run(&path)?;
        }
        Commands::Validate {
            stype,
            payload,
            schema,
            registry,
        } => {
            commands::validate::run(&stype, &payload, schema.as_deref(), &registry)?;
        }
        Commands::AddTool {
            tool_id,
            args_stype,
            returns_stype,
            profile,
            policy,
        } => {
            commands::add_tool::run(&tool_id, &args_stype, &returns_stype, profile, policy)?;
        }
        Commands::AddProfile { name, profile } => {
            commands::add_profile::run(&name, &profile)?;
        }
        Commands::Hash { payload } => {
            commands::hash::run(&payload)?;
        }
        Commands::Qom { command } => match command {
            QomCommands::Evaluate {
                profile,
                payload,
                response,
                registry,
            } => {
                commands::qom_evaluate::run(&profile, &payload, response.as_deref(), &registry)?;
            }
        },
        Commands::Conformance { registry, filter } => {
            commands::conformance::run(&registry, filter.as_deref(), cli.verbose)?;
        }
    }

    Ok(())
}
