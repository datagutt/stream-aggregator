//! Router configuration

use axum::{
    middleware,
    routing::{delete, get, post},
    Router,
};
use std::collections::HashMap;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use stream_aggregator_core::traits::{StreamStore, PlatformProvider};

use crate::handlers::*;
use crate::middleware::{auth_middleware, AuthConfig};

/// Create the main API router
pub fn create_router(
    store: Arc<dyn StreamStore>,
    providers: HashMap<String, Arc<dyn PlatformProvider>>,
) -> Router {
    create_router_with_auth(store, providers, AuthConfig::default())
}

/// Create the main API router with authentication
pub fn create_router_with_auth(
    store: Arc<dyn StreamStore>,
    providers: HashMap<String, Arc<dyn PlatformProvider>>,
    auth_config: AuthConfig,
) -> Router {
    let state = AppState {
        store,
        providers: Arc::new(providers),
    };

    Router::new()
        // Health check (always public)
        .route("/health", get(health_check))
        .route("/api/v1/health", get(health_check))
        // Streams endpoints (public reads by default)
        .route("/api/v1/streams", get(list_streams))
        .route("/api/v1/streams/:id", get(get_stream))
        // Streamers endpoints (writes require auth)
        .route("/api/v1/streamers", get(list_streamers).post(add_streamer))
        .route(
            "/api/v1/streamers/:platform/:user_id",
            delete(remove_streamer),
        )
        // Platforms endpoint (public by default)
        .route("/api/v1/platforms", get(list_platforms))
        .with_state(state)
        // Apply authentication middleware
        .layer(middleware::from_fn_with_state(
            auth_config,
            auth_middleware,
        ))
        // Apply CORS and tracing
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}
