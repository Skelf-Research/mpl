//! Registry API handlers

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::fs;

use crate::error::RegistryError;
use crate::state::RegistryState;

/// Health check response
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
}

/// Health check handler
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        service: "mpl-registry-api".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// SType metadata response
#[derive(Clone, Serialize)]
pub struct StypeMetadata {
    pub stype: String,
    pub namespace: String,
    pub domain: String,
    pub name: String,
    pub version: u32,
    pub schema_url: String,
    pub urn: String,
}

/// Get schema for an SType
pub async fn get_schema(
    State(state): State<Arc<RegistryState>>,
    Path((namespace, domain, name, version)): Path<(String, String, String, String)>,
) -> Result<Json<Value>, RegistryError> {
    // Parse version
    let version_num = version
        .strip_prefix('v')
        .and_then(|v| v.parse::<u32>().ok())
        .ok_or_else(|| RegistryError::InvalidFormat(format!("Invalid version: {}", version)))?;

    let stype_id = format!("{}.{}.{}.v{}", namespace, domain, name, version_num);

    // Check cache first
    if let Some(schema) = state.cache.get(&stype_id) {
        return Ok(Json(schema));
    }

    // Load from filesystem
    let schema_path = state.schema_path(&namespace, &domain, &name, version_num);

    if !schema_path.exists() {
        return Err(RegistryError::NotFound(stype_id));
    }

    let content = fs::read_to_string(&schema_path).await?;
    let schema: Value = serde_json::from_str(&content)
        .map_err(|e| RegistryError::SchemaError(e.to_string()))?;

    // Cache the schema
    state.cache.insert(stype_id, schema.clone());

    Ok(Json(schema))
}

/// Get SType metadata
pub async fn get_stype_metadata(
    State(state): State<Arc<RegistryState>>,
    Path((namespace, domain, name, version)): Path<(String, String, String, String)>,
) -> Result<Json<StypeMetadata>, RegistryError> {
    let version_num = version
        .strip_prefix('v')
        .and_then(|v| v.parse::<u32>().ok())
        .ok_or_else(|| RegistryError::InvalidFormat(format!("Invalid version: {}", version)))?;

    let stype_path = state.stype_path(&namespace, &domain, &name, version_num);

    if !stype_path.exists() {
        return Err(RegistryError::NotFound(format!(
            "{}.{}.{}.v{}",
            namespace, domain, name, version_num
        )));
    }

    let stype_id = format!("{}.{}.{}.v{}", namespace, domain, name, version_num);

    Ok(Json(StypeMetadata {
        stype: stype_id.clone(),
        namespace: namespace.clone(),
        domain: domain.clone(),
        name: name.clone(),
        version: version_num,
        schema_url: format!("/stypes/{}/{}/{}/v{}/schema", namespace, domain, name, version_num),
        urn: format!("urn:stype:{}", stype_id),
    }))
}

/// List query parameters
#[derive(Deserialize)]
pub struct ListQuery {
    pub namespace: Option<String>,
    pub domain: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// List STypes
#[derive(Serialize)]
pub struct ListResponse {
    pub stypes: Vec<StypeMetadata>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
}

/// List all STypes
pub async fn list_stypes(
    State(state): State<Arc<RegistryState>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<ListResponse>, RegistryError> {
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    let stypes_dir = state.registry_path.join("stypes");
    let mut stypes = Vec::new();

    // Walk the registry directory
    if let Ok(mut entries) = fs::read_dir(&stypes_dir).await {
        while let Ok(Some(ns_entry)) = entries.next_entry().await {
            let ns_name = ns_entry.file_name().to_string_lossy().to_string();

            // Filter by namespace if specified
            if let Some(ref filter_ns) = query.namespace {
                if !ns_name.starts_with(filter_ns) {
                    continue;
                }
            }

            if let Ok(mut domain_entries) = fs::read_dir(ns_entry.path()).await {
                while let Ok(Some(domain_entry)) = domain_entries.next_entry().await {
                    let domain_name = domain_entry.file_name().to_string_lossy().to_string();

                    // Filter by domain if specified
                    if let Some(ref filter_domain) = query.domain {
                        if !domain_name.starts_with(filter_domain) {
                            continue;
                        }
                    }

                    if let Ok(mut name_entries) = fs::read_dir(domain_entry.path()).await {
                        while let Ok(Some(name_entry)) = name_entries.next_entry().await {
                            let type_name = name_entry.file_name().to_string_lossy().to_string();

                            if let Ok(mut version_entries) = fs::read_dir(name_entry.path()).await {
                                while let Ok(Some(version_entry)) = version_entries.next_entry().await {
                                    let version_str = version_entry.file_name().to_string_lossy().to_string();

                                    if let Some(version_num) = version_str
                                        .strip_prefix('v')
                                        .and_then(|v| v.parse::<u32>().ok())
                                    {
                                        let stype_id = format!(
                                            "{}.{}.{}.v{}",
                                            ns_name, domain_name, type_name, version_num
                                        );

                                        stypes.push(StypeMetadata {
                                            stype: stype_id.clone(),
                                            namespace: ns_name.clone(),
                                            domain: domain_name.clone(),
                                            name: type_name.clone(),
                                            version: version_num,
                                            schema_url: format!(
                                                "/stypes/{}/{}/{}/v{}/schema",
                                                ns_name, domain_name, type_name, version_num
                                            ),
                                            urn: format!("urn:stype:{}", stype_id),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Sort by SType ID
    stypes.sort_by(|a, b| a.stype.cmp(&b.stype));

    let total = stypes.len();
    let stypes: Vec<_> = stypes.into_iter().skip(offset).take(limit).collect();

    Ok(Json(ListResponse {
        stypes,
        total,
        limit,
        offset,
    }))
}

/// Search query
#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub limit: Option<usize>,
}

/// Search STypes
pub async fn search_stypes(
    State(state): State<Arc<RegistryState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<ListResponse>, RegistryError> {
    let limit = query.limit.unwrap_or(20).min(50);
    let search_term = query.q.to_lowercase();

    // Get all stypes and filter
    let list_result = list_stypes(
        State(state),
        Query(ListQuery {
            namespace: None,
            domain: None,
            limit: Some(1000),
            offset: Some(0),
        }),
    )
    .await?;

    let matches: Vec<_> = list_result
        .0
        .stypes
        .into_iter()
        .filter(|s| {
            s.stype.to_lowercase().contains(&search_term)
                || s.name.to_lowercase().contains(&search_term)
                || s.domain.to_lowercase().contains(&search_term)
        })
        .take(limit)
        .collect();

    let total = matches.len();

    Ok(Json(ListResponse {
        stypes: matches,
        total,
        limit,
        offset: 0,
    }))
}

/// Cache statistics
#[derive(Serialize)]
pub struct CacheStatsResponse {
    pub entry_count: u64,
    pub weighted_size: u64,
}

/// Get cache stats
pub async fn cache_stats(
    State(state): State<Arc<RegistryState>>,
) -> Json<CacheStatsResponse> {
    let stats = state.cache.stats();
    Json(CacheStatsResponse {
        entry_count: stats.entry_count,
        weighted_size: stats.weighted_size,
    })
}
