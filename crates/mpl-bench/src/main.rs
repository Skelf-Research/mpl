//! qom-eval: research harness for the QoM benchmark.
//!
//! Reads a JSONL file of cases — each a `MetricContext` plus a ground-truth
//! label and an optional self-contained JSON Schema — runs the real
//! `QomComputer` + `SchemaValidator` from `mpl_core`, and emits a JSONL of
//! computed metrics and profile pass/fail decisions for downstream analysis.
//!
//! Usage: qom-eval <cases.jsonl> [results.jsonl]
//! (writes to stdout if no output path is given)

use std::fs;
use std::io::{BufRead, BufReader, Write};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use mpl_core::metrics::{MetricContext, QomComputer};
use mpl_core::qom::QomProfile;
use mpl_core::validation::SchemaValidator;

/// One benchmark case: the metric context, a ground-truth label describing the
/// injected failure (if any), an optional JSON Schema for Schema-Fidelity, and
/// the QoM profile to enforce against.
#[derive(Deserialize)]
struct Case {
    case_id: String,
    #[serde(default)]
    label: Value,
    #[serde(default)]
    schema: Option<Value>,
    context: MetricContext,
    #[serde(default = "default_profile")]
    profile: String,
}

fn default_profile() -> String {
    "comprehensive".to_string()
}

/// One result row: computed metrics + enforcement decision, carrying the label
/// through so analysis can compute detection precision/recall per failure class.
#[derive(Serialize)]
struct Output {
    case_id: String,
    label: Value,
    profile: String,
    metrics: Value,
    meets_profile: bool,
    failures: Value,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<String>,
}

fn profile_by_name(name: &str) -> QomProfile {
    match name {
        "basic" => QomProfile::basic(),
        "strict-argcheck" | "strict_argcheck" => QomProfile::strict_argcheck(),
        "outcome" => QomProfile::outcome(),
        _ => QomProfile::comprehensive(),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: qom-eval <cases.jsonl> [results.jsonl]");
        std::process::exit(2);
    }

    let file = fs::File::open(&args[1]).unwrap_or_else(|e| {
        eprintln!("error opening {}: {}", args[1], e);
        std::process::exit(1);
    });
    let reader = BufReader::new(file);

    let mut out: Box<dyn Write> = if args.len() >= 3 {
        Box::new(fs::File::create(&args[2]).unwrap_or_else(|e| {
            eprintln!("error creating {}: {}", args[2], e);
            std::process::exit(1);
        }))
    } else {
        Box::new(std::io::stdout())
    };

    // Compute every dimension we can; each is only produced when the relevant
    // context field is present (assertions->IC, sources->G, prev->DJ, etc.).
    let computer = QomComputer::new()
        .with_ic(true)
        .with_toc(true)
        .with_groundedness(true)
        .with_determinism(true)
        .with_ontology(true);

    let (mut n_ok, mut n_bad) = (0usize, 0usize);

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("read error: {}", e);
                continue;
            }
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let case: Case = match serde_json::from_str(line) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("skipping malformed case: {}", e);
                n_bad += 1;
                continue;
            }
        };

        let mut errors: Vec<String> = Vec::new();

        // Schema Fidelity: QomComputer treats SF as an upstream concern, so we
        // run the validator here against the case's self-contained schema.
        let mut schema_fidelity = 1.0;
        if let Some(schema) = &case.schema {
            let mut validator = SchemaValidator::new();
            match validator.register_json(&case.context.stype, &schema.to_string()) {
                Ok(()) => match validator.validate(&case.context.stype, &case.context.payload) {
                    Ok(res) => schema_fidelity = if res.valid { 1.0 } else { 0.0 },
                    Err(e) => errors.push(format!("schema validation: {}", e)),
                },
                Err(e) => errors.push(format!("schema registration: {}", e)),
            }
        }

        let result = computer.compute(&case.context);
        let mut metrics = result.metrics;
        metrics.schema_fidelity = schema_fidelity;
        errors.extend(result.errors);

        let eval = profile_by_name(&case.profile).evaluate(&metrics);

        let output = Output {
            case_id: case.case_id,
            label: case.label,
            profile: case.profile,
            metrics: serde_json::to_value(&metrics).unwrap_or(Value::Null),
            meets_profile: eval.meets_profile,
            failures: serde_json::to_value(&eval.failures).unwrap_or(Value::Null),
            errors,
        };

        match serde_json::to_string(&output) {
            Ok(s) => {
                let _ = writeln!(out, "{}", s);
                n_ok += 1;
            }
            Err(e) => eprintln!("serialize error for {}: {}", output.case_id, e),
        }
    }

    eprintln!("processed {} cases ({} malformed)", n_ok, n_bad);
}
