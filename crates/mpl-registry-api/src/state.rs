//! Registry API state

use std::path::PathBuf;
use std::sync::Arc;

use crate::cache::SchemaCache;

/// Shared state for the registry API
#[derive(Clone)]
pub struct RegistryState {
    /// Path to the registry directory
    pub registry_path: PathBuf,
    /// Schema cache
    pub cache: Arc<SchemaCache>,
}

impl RegistryState {
    /// Create new registry state
    pub fn new(registry_path: PathBuf) -> Self {
        Self {
            registry_path,
            cache: Arc::new(SchemaCache::new()),
        }
    }

    /// Get the path for an SType
    pub fn stype_path(&self, namespace: &str, domain: &str, name: &str, version: u32) -> PathBuf {
        self.registry_path
            .join("stypes")
            .join(namespace)
            .join(domain)
            .join(name)
            .join(format!("v{}", version))
    }

    /// Get the schema path for an SType
    pub fn schema_path(&self, namespace: &str, domain: &str, name: &str, version: u32) -> PathBuf {
        self.stype_path(namespace, domain, name, version)
            .join("schema.json")
    }

    /// Get the examples directory for an SType
    pub fn examples_path(
        &self,
        namespace: &str,
        domain: &str,
        name: &str,
        version: u32,
    ) -> PathBuf {
        self.stype_path(namespace, domain, name, version)
            .join("examples")
    }
}
