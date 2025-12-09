//! # stream-aggregator-api
//!
//! HTTP API layer for StreamAggregator using axum.
//!
//! This crate provides:
//! - REST API endpoints
//! - Request/response types
//! - Error handling
//! - Middleware (CORS, logging, authentication)

mod handlers;
mod middleware;
mod responses;
mod router;

pub use middleware::AuthConfig;
pub use router::{create_router, create_router_with_auth};
pub use responses::*;
