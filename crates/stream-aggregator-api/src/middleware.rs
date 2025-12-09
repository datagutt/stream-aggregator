//! API middleware (authentication, rate limiting, etc.)

use axum::{
    extract::{Request, State},
    http::{HeaderMap, Method, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use std::collections::HashSet;
use tracing::{debug, warn};

use crate::responses::ErrorResponse;

/// Configuration for API key authentication
#[derive(Clone)]
pub struct AuthConfig {
    /// Valid API keys
    pub api_keys: HashSet<String>,
    /// Whether authentication is enabled
    pub enabled: bool,
    /// Whether to require auth for read operations
    pub require_auth_for_reads: bool,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            api_keys: HashSet::new(),
            enabled: false,
            require_auth_for_reads: false,
        }
    }
}

impl AuthConfig {
    /// Create a new auth config with API keys
    pub fn new(api_keys: Vec<String>) -> Self {
        Self {
            api_keys: api_keys.into_iter().collect(),
            enabled: true,
            require_auth_for_reads: false,
        }
    }

    /// Require authentication for all requests (including reads)
    pub fn require_all(mut self) -> Self {
        self.require_auth_for_reads = true;
        self
    }
}

/// Extract API key from request headers or query parameters
fn extract_api_key(headers: &HeaderMap, uri: &axum::http::Uri) -> Option<String> {
    // Try X-API-Key header first
    if let Some(key) = headers.get("X-API-Key") {
        if let Ok(key_str) = key.to_str() {
            return Some(key_str.to_string());
        }
    }

    // Try Authorization: Bearer <key> header
    if let Some(auth) = headers.get("Authorization") {
        if let Ok(auth_str) = auth.to_str() {
            if let Some(key) = auth_str.strip_prefix("Bearer ") {
                return Some(key.to_string());
            }
        }
    }

    // Try ?api_key= query parameter
    if let Some(query) = uri.query() {
        for param in query.split('&') {
            if let Some((key, value)) = param.split_once('=') {
                if key == "api_key" {
                    return Some(value.to_string());
                }
            }
        }
    }

    None
}

/// Check if a path requires authentication
fn requires_auth(method: &Method, path: &str, config: &AuthConfig) -> bool {
    // If auth is disabled, nothing requires auth
    if !config.enabled {
        return false;
    }

    // Public endpoints (always accessible)
    let public_endpoints = [
        "/health",
        "/api/v1/health",
        "/api/v1/health/ready",
        "/api/v1/health/live",
    ];

    if public_endpoints.iter().any(|&ep| path.starts_with(ep)) {
        return false;
    }

    // Read-only endpoints (public if require_auth_for_reads is false)
    if method == Method::GET && !config.require_auth_for_reads {
        let read_only_prefixes = [
            "/api/v1/streams",
            "/api/v1/platforms",
            "/api/v1/stats",
            "/api/v1/groups",
        ];

        if read_only_prefixes.iter().any(|&prefix| path.starts_with(prefix)) {
            return false;
        }
    }

    // Everything else requires authentication
    true
}

/// Authentication middleware
pub async fn auth_middleware(
    State(config): State<AuthConfig>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let method = request.method().clone();
    let path = request.uri().path().to_string();

    // Check if this endpoint requires authentication
    if !requires_auth(&method, &path, &config) {
        debug!(method = %method, path = %path, "Public endpoint, skipping auth");
        return Ok(next.run(request).await);
    }

    // Extract API key
    let api_key = extract_api_key(request.headers(), request.uri());

    match api_key {
        Some(key) if config.api_keys.contains(&key) => {
            debug!(method = %method, path = %path, "API key valid");
            Ok(next.run(request).await)
        }
        Some(_) => {
            warn!(method = %method, path = %path, "Invalid API key");
            Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "UNAUTHORIZED",
                    "Invalid API key",
                )),
            ))
        }
        None => {
            warn!(method = %method, path = %path, "Missing API key");
            Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "UNAUTHORIZED",
                    "Missing API key. Provide via X-API-Key header or ?api_key= query parameter",
                )),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderValue, Uri};

    #[test]
    fn test_extract_api_key_from_header() {
        let mut headers = HeaderMap::new();
        headers.insert("X-API-Key", HeaderValue::from_static("test-key-123"));

        let uri = Uri::from_static("/api/v1/streams");
        let key = extract_api_key(&headers, &uri);

        assert_eq!(key, Some("test-key-123".to_string()));
    }

    #[test]
    fn test_extract_api_key_from_bearer() {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", HeaderValue::from_static("Bearer test-key-456"));

        let uri = Uri::from_static("/api/v1/streams");
        let key = extract_api_key(&headers, &uri);

        assert_eq!(key, Some("test-key-456".to_string()));
    }

    #[test]
    fn test_extract_api_key_from_query() {
        let headers = HeaderMap::new();
        let uri = Uri::from_static("/api/v1/streams?api_key=test-key-789");
        let key = extract_api_key(&headers, &uri);

        assert_eq!(key, Some("test-key-789".to_string()));
    }

    #[test]
    fn test_requires_auth_public_endpoints() {
        let config = AuthConfig::new(vec!["key1".to_string()]);

        assert!(!requires_auth(&Method::GET, "/health", &config));
        assert!(!requires_auth(&Method::GET, "/api/v1/health", &config));
    }

    #[test]
    fn test_requires_auth_read_endpoints() {
        let config = AuthConfig::new(vec!["key1".to_string()]);

        // Read endpoints don't require auth by default
        assert!(!requires_auth(&Method::GET, "/api/v1/streams", &config));
        assert!(!requires_auth(&Method::GET, "/api/v1/platforms", &config));

        // Write endpoints require auth
        assert!(requires_auth(&Method::POST, "/api/v1/streamers", &config));
        assert!(requires_auth(&Method::DELETE, "/api/v1/streamers/twitch/user", &config));
    }

    #[test]
    fn test_requires_auth_all_when_configured() {
        let config = AuthConfig::new(vec!["key1".to_string()]).require_all();

        // Now read endpoints require auth too
        assert!(requires_auth(&Method::GET, "/api/v1/streams", &config));
        assert!(requires_auth(&Method::GET, "/api/v1/platforms", &config));

        // Health endpoints still public
        assert!(!requires_auth(&Method::GET, "/health", &config));
    }

    #[test]
    fn test_auth_disabled() {
        let config = AuthConfig::default(); // disabled by default

        // Nothing requires auth when disabled
        assert!(!requires_auth(&Method::GET, "/api/v1/streams", &config));
        assert!(!requires_auth(&Method::POST, "/api/v1/streamers", &config));
        assert!(!requires_auth(&Method::DELETE, "/api/v1/streamers/twitch/user", &config));
    }
}
