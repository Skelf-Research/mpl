//! MPL Registry API
//!
//! REST service for SType discovery and schema retrieval.

pub mod cache;
pub mod error;
pub mod handlers;
pub mod routes;
pub mod state;

pub use routes::create_router;
pub use state::RegistryState;
