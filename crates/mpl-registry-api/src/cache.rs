//! Schema caching with TTL

use moka::sync::Cache;
use serde_json::Value;
use std::time::Duration;

/// Cache for schemas with automatic expiration
pub struct SchemaCache {
    schemas: Cache<String, Value>,
}

impl SchemaCache {
    /// Create a new schema cache
    pub fn new() -> Self {
        Self {
            schemas: Cache::builder()
                .max_capacity(1000)
                .time_to_live(Duration::from_secs(300)) // 5 minute TTL
                .time_to_idle(Duration::from_secs(60))   // 1 minute idle
                .build(),
        }
    }

    /// Get a schema from cache
    pub fn get(&self, stype: &str) -> Option<Value> {
        self.schemas.get(stype)
    }

    /// Insert a schema into cache
    pub fn insert(&self, stype: String, schema: Value) {
        self.schemas.insert(stype, schema);
    }

    /// Check if schema is cached
    pub fn contains(&self, stype: &str) -> bool {
        self.schemas.contains_key(stype)
    }

    /// Invalidate a schema
    pub fn invalidate(&self, stype: &str) {
        self.schemas.invalidate(stype);
    }

    /// Clear all cached schemas
    pub fn clear(&self) {
        self.schemas.invalidate_all();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entry_count: self.schemas.entry_count(),
            weighted_size: self.schemas.weighted_size(),
        }
    }
}

impl Default for SchemaCache {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CacheStats {
    pub entry_count: u64,
    pub weighted_size: u64,
}
