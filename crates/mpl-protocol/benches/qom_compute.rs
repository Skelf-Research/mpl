//! Benchmark `QomComputer::compute` for a realistic MetricContext — the
//! per-call cost the proxy pays in strict mode. Schema fidelity is set
//! upstream by SchemaValidator so it isn't recomputed here; this measures
//! IC (CEL assertions) + OA (ontology rules) which are what the comprehensive
//! profile pays for on every payload.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mpl_core::assertions::{Assertion, AssertionSet};
use mpl_core::metrics::{MetricContext, OntologyConstraints, QomComputer};
use serde_json::json;

fn ctx() -> MetricContext {
    let assertions = AssertionSet::new(vec![
        Assertion::new(
            "amount_positive",
            "payload.amount > 0",
            "Amount must be positive",
        ),
        Assertion::new(
            "currency_valid",
            "payload.currency in ['USD','EUR','GBP']",
            "Unsupported currency",
        ),
        Assertion::new(
            "amount_within_limit",
            "payload.amount <= 1000000",
            "Amount exceeds limit",
        ),
    ]);
    let mut ontology = OntologyConstraints::default();
    ontology
        .allowed_values
        .insert("status".into(), vec![json!("active"), json!("settled")]);

    MetricContext::new(
        "org.finance.Transfer.v1",
        json!({
            "amount": 100,
            "currency": "USD",
            "account_id": "ACME-1234",
            "status": "settled",
            "request_id": "req-00001"
        }),
    )
    .with_assertions(assertions)
    .with_ontology(ontology)
}

fn bench_qom_compute(c: &mut Criterion) {
    let computer = QomComputer::new().with_ic(true).with_ontology(true);
    let ctx = ctx();
    c.bench_function("qom_compute/finance_transfer", |b| {
        b.iter(|| {
            let r = computer.compute(black_box(&ctx));
            black_box(r)
        });
    });
}

criterion_group!(benches, bench_qom_compute);
criterion_main!(benches);
