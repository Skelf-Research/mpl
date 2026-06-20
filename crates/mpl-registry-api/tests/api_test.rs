//! Integration tests for the registry API.
//!
//! Exercises the full router via tower::ServiceExt::oneshot, plus direct
//! state/error tests where the HTTP layer would obscure the behavior.
//!
//! Layout under the temp registry dir mirrors production:
//!   stypes/<namespace>/<domain>/<name>/v<version>/schema.json

use std::fs;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tempfile::TempDir;
use tower::ServiceExt;

use mpl_registry_api::{create_router, state::RegistryState};

/// Build a temp registry containing one or more known STypes and return the
/// router state ready to mount under axum. Each entry is
/// `(namespace, domain, name, version, schema_json)`.
fn fixture(entries: &[(&str, &str, &str, u32, &str)]) -> (TempDir, Arc<RegistryState>) {
    let tmp = TempDir::new().expect("tempdir");
    for (ns, dom, name, ver, schema) in entries {
        let dir = tmp
            .path()
            .join("stypes")
            .join(ns)
            .join(dom)
            .join(name)
            .join(format!("v{}", ver));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("schema.json"), schema).unwrap();
    }
    let state = Arc::new(RegistryState::new(tmp.path().to_path_buf()));
    (tmp, state)
}

async fn get(router: axum::Router, path: &str) -> (StatusCode, Value) {
    let resp = router
        .oneshot(
            Request::builder()
                .uri(path)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("oneshot");
    let status = resp.status();
    let body = resp.into_body().collect().await.expect("body").to_bytes();
    let v: Value = if body.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&body).expect("json")
    };
    (status, v)
}

// ---- health ---------------------------------------------------------------

#[tokio::test]
async fn health_endpoint_returns_healthy() {
    let (_tmp, state) = fixture(&[]);
    let (status, body) = get(create_router(state), "/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "healthy");
    assert_eq!(body["service"], "mpl-registry-api");
}

// ---- get_schema -----------------------------------------------------------

#[tokio::test]
async fn get_schema_returns_404_for_unknown_stype() {
    let (_tmp, state) = fixture(&[]);
    let (status, body) = get(
        create_router(state),
        "/stypes/org/finance/Missing/v1/schema",
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "E-NOT-FOUND");
}

#[tokio::test]
async fn get_schema_returns_400_for_invalid_version_format() {
    let (_tmp, state) = fixture(&[]);
    let (status, body) = get(
        create_router(state),
        "/stypes/org/finance/Transfer/notaversion/schema",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "E-INVALID-FORMAT");
}

#[tokio::test]
async fn get_schema_returns_the_registered_schema() {
    let schema = r#"{"type":"object","required":["amount"]}"#;
    let (_tmp, state) = fixture(&[("org", "finance", "Transfer", 1, schema)]);
    let (status, body) = get(
        create_router(state),
        "/stypes/org/finance/Transfer/v1/schema",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["type"], "object");
    assert_eq!(body["required"], serde_json::json!(["amount"]));
}

#[tokio::test]
async fn get_schema_is_cached_after_first_hit() {
    let schema = r#"{"type":"object"}"#;
    let (_tmp, state) = fixture(&[("org", "finance", "Transfer", 1, schema)]);
    // Prime the cache via one request.
    let _ = get(
        create_router(state.clone()),
        "/stypes/org/finance/Transfer/v1/schema",
    )
    .await;
    // Direct cache check: the stype_id key should now be present.
    assert!(state.cache.get("org.finance.Transfer.v1").is_some());
}

// ---- get_stype_metadata ---------------------------------------------------

#[tokio::test]
async fn get_metadata_returns_404_for_missing_dir() {
    let (_tmp, state) = fixture(&[]);
    let (status, _) = get(create_router(state), "/stypes/org/finance/Nope/v1").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_metadata_returns_canonical_fields() {
    let (_tmp, state) = fixture(&[("org", "finance", "Transfer", 2, "{}")]);
    let (status, body) = get(create_router(state), "/stypes/org/finance/Transfer/v2").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["stype"], "org.finance.Transfer.v2");
    assert_eq!(body["namespace"], "org");
    assert_eq!(body["domain"], "finance");
    assert_eq!(body["name"], "Transfer");
    assert_eq!(body["version"], 2);
    assert_eq!(body["schema_url"], "/stypes/org/finance/Transfer/v2/schema");
    assert_eq!(body["urn"], "urn:stype:org.finance.Transfer.v2");
}

// ---- list / search --------------------------------------------------------

#[tokio::test]
async fn list_stypes_empty_registry_returns_empty() {
    let (_tmp, state) = fixture(&[]);
    let (status, body) = get(create_router(state), "/stypes").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 0);
    assert!(body["stypes"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn list_stypes_walks_the_registry_tree() {
    let (_tmp, state) = fixture(&[
        ("org", "finance", "Transfer", 1, "{}"),
        ("org", "finance", "Refund", 1, "{}"),
        ("org", "calendar", "Event", 1, "{}"),
    ]);
    let (status, body) = get(create_router(state), "/stypes").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 3);
    let ids: Vec<String> = body["stypes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["stype"].as_str().unwrap().to_string())
        .collect();
    // sorted
    assert_eq!(
        ids,
        vec![
            "org.calendar.Event.v1",
            "org.finance.Refund.v1",
            "org.finance.Transfer.v1",
        ]
    );
}

#[tokio::test]
async fn list_stypes_respects_namespace_filter() {
    let (_tmp, state) = fixture(&[
        ("org", "finance", "Transfer", 1, "{}"),
        ("eval", "rag", "Answer", 1, "{}"),
    ]);
    let (_status, body) = get(create_router(state), "/stypes?namespace=eval").await;
    assert_eq!(body["total"], 1);
    assert_eq!(body["stypes"][0]["stype"], "eval.rag.Answer.v1");
}

#[tokio::test]
async fn list_stypes_clamps_limit_to_100() {
    let (_tmp, state) = fixture(&[("org", "finance", "Transfer", 1, "{}")]);
    let (_status, body) = get(create_router(state), "/stypes?limit=99999").await;
    assert_eq!(body["limit"], 100);
}

#[tokio::test]
async fn search_matches_substring_in_id_or_name_or_domain() {
    let (_tmp, state) = fixture(&[
        ("org", "finance", "Transfer", 1, "{}"),
        ("org", "calendar", "Event", 1, "{}"),
    ]);
    let (_status, body) = get(create_router(state), "/search?q=trans").await;
    assert_eq!(body["total"], 1);
    assert_eq!(body["stypes"][0]["name"], "Transfer");
}

// ---- cache stats ----------------------------------------------------------

#[tokio::test]
async fn cache_stats_returns_zero_counts_on_fresh_state() {
    let (_tmp, state) = fixture(&[]);
    let (status, body) = get(create_router(state), "/cache/stats").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["entry_count"].is_number());
    assert!(body["weighted_size"].is_number());
}

// ---- RegistryState path helpers (unit) -----------------------------------

#[test]
fn stype_path_assembles_namespace_domain_name_version() {
    let tmp = TempDir::new().unwrap();
    let s = RegistryState::new(tmp.path().to_path_buf());
    let p = s.stype_path("org", "finance", "Transfer", 1);
    assert!(p.ends_with("stypes/org/finance/Transfer/v1"));
}

#[test]
fn schema_path_ends_in_schema_json() {
    let tmp = TempDir::new().unwrap();
    let s = RegistryState::new(tmp.path().to_path_buf());
    let p = s.schema_path("org", "finance", "Transfer", 1);
    assert!(p.ends_with("schema.json"));
}

#[test]
fn examples_path_ends_in_examples_dir() {
    let tmp = TempDir::new().unwrap();
    let s = RegistryState::new(tmp.path().to_path_buf());
    let p = s.examples_path("org", "finance", "Transfer", 1);
    assert!(p.ends_with("examples"));
}
