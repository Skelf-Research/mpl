//! Registry API routes

use axum::{routing::get, Router};
use std::sync::Arc;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use crate::handlers;
use crate::state::RegistryState;

/// Create the API router
pub fn create_router(state: Arc<RegistryState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Health check
        .route("/health", get(handlers::health))
        // SType operations
        .route(
            "/stypes/:namespace/:domain/:name/:version",
            get(handlers::get_stype_metadata),
        )
        .route(
            "/stypes/:namespace/:domain/:name/:version/schema",
            get(handlers::get_schema),
        )
        // List and search
        .route("/stypes", get(handlers::list_stypes))
        .route("/search", get(handlers::search_stypes))
        // Cache management
        .route("/cache/stats", get(handlers::cache_stats))
        // Middleware
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
