//! MPL middleware for request/response processing
//!
//! # Architecture Note
//!
//! MPL validation is implemented in the request handlers (`proxy.rs`) rather than
//! as Tower middleware. This is the standard pattern for axum when body inspection
//! is required, because:
//!
//! 1. Request bodies can only be consumed once - middleware would need to buffer
//!    the entire body, parse it, then reconstruct it for downstream handlers
//! 2. Validation logic is tightly coupled with routing (different paths may have
//!    different validation requirements)
//! 3. Response enrichment (adding MPL headers) is simpler in handlers
//!
//! The `proxy_handler` in `handlers.rs` calls `ProxyState::forward_request()` which:
//! - Parses MPL envelopes from body or X-MPL-SType header
//! - Validates against registered schemas
//! - Evaluates QoM profiles
//! - Verifies semantic hashes
//! - Returns 400 in strict mode for failures
//! - Adds X-MPL-QoM-Result headers to responses
//!
//! This middleware layer is available for future use cases like:
//! - WebSocket connection upgrade interception
//! - Request logging/tracing
//! - Rate limiting based on SType

use std::sync::Arc;
use std::task::{Context, Poll};

use axum::body::Body;
use axum::http::{Request, Response};
use tower::{Layer, Service};

use crate::proxy::ProxyState;

/// MPL middleware layer for future extensibility
#[derive(Clone)]
pub struct MplMiddleware {
    #[allow(dead_code)]
    state: Arc<ProxyState>,
}

impl MplMiddleware {
    pub fn new(state: Arc<ProxyState>) -> Self {
        Self { state }
    }
}

impl<S> Layer<S> for MplMiddleware {
    type Service = MplMiddlewareService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MplMiddlewareService {
            inner,
            #[allow(dead_code)]
            state: self.state.clone(),
        }
    }
}

/// MPL middleware service
#[derive(Clone)]
pub struct MplMiddlewareService<S> {
    inner: S,
    #[allow(dead_code)]
    state: Arc<ProxyState>,
}

impl<S> Service<Request<Body>> for MplMiddlewareService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        // Pass through - validation is handled in request handlers
        // See module-level documentation for architecture rationale
        self.inner.call(request)
    }
}
