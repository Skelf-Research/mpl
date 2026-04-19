//! MPL Sidecar Proxy Library
//!
//! Provides the core components for the MPL proxy that intercepts
//! MCP/A2A traffic and adds MPL validation.

pub mod config;
pub mod handlers;
pub mod metrics;
pub mod middleware;
pub mod proxy;
