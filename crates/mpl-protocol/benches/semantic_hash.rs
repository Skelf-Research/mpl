//! Benchmark the protocol-side canonicalization + BLAKE3 hash that the proxy
//! computes on every envelope. Three sizes: a typical tool-call payload, a
//! medium settlement-style payload, and a big RAG-answer payload with a long
//! string body.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use mpl_core::hash::semantic_hash;
use serde_json::{json, Value};

fn small_payload() -> Value {
    json!({
        "amount": 100,
        "currency": "USD",
        "account_id": "ACME-1234",
        "status": "settled",
        "request_id": "req-00001"
    })
}

fn medium_payload() -> Value {
    json!({
        "transactions": (0..32).map(|i| json!({
            "id": format!("tx-{:06}", i),
            "amount": (i as f64) * 12.5,
            "currency": "USD",
            "ts": 1700000000_u64 + i as u64,
        })).collect::<Vec<_>>(),
        "summary": {
            "count": 32,
            "total": 32.0_f64 * 12.5 * 15.5,
            "settlement_id": "S-2026-06-19-001",
        }
    })
}

fn large_payload() -> Value {
    json!({
        "answer": "x".repeat(8 * 1024),                  // ~8 KB body
        "sources": (0..16).map(|i| json!({
            "id": format!("doc-{:04}", i),
            "content": "y".repeat(256),
            "confidence": 0.5 + (i as f64) * 0.01,
        })).collect::<Vec<_>>(),
        "confidence_band": "high",
    })
}

fn bench_semantic_hash(c: &mut Criterion) {
    let mut group = c.benchmark_group("semantic_hash");
    for (label, payload) in [
        ("small", small_payload()),
        ("medium", medium_payload()),
        ("large", large_payload()),
    ] {
        let size = serde_json::to_string(&payload).unwrap().len();
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(label), &payload, |b, p| {
            b.iter(|| semantic_hash(black_box(p)).expect("hash"));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_semantic_hash);
criterion_main!(benches);
