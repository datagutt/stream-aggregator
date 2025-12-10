//! Error types for StreamAggregator

use thiserror::Error;

/// Errors that can occur in platform providers
#[derive(Debug, Clone, Error)]
pub enum ProviderError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    HttpError(String),

    /// Failed to parse response
    #[error("Failed to parse response: {0}")]
    ParseError(String),

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthError(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    /// Streamer not found
    #[error("Streamer not found: {0}")]
    StreamerNotFound(String),

    /// Discovery not supported by this provider
    #[error("Discovery not supported by this provider")]
    DiscoveryNotSupported,

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    /// Platform-specific error
    #[error("Platform error: {0}")]
    PlatformError(String),

    /// Internal error
    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Errors that can occur in storage operations
#[derive(Debug, Error)]
pub enum StoreError {
    /// Database connection failed
    #[error("Database connection failed: {0}")]
    ConnectionError(String),

    /// Database query failed
    #[error("Database query failed: {0}")]
    QueryError(String),

    /// Item not found
    #[error("Item not found: {0}")]
    NotFound(String),

    /// Item already exists
    #[error("Item already exists: {0}")]
    AlreadyExists(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Invalid query parameters
    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    /// Internal error
    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Errors that can occur in the scheduler
#[derive(Debug, Error)]
pub enum SchedulerError {
    /// Provider error
    #[error("Provider error: {0}")]
    ProviderError(#[from] ProviderError),

    /// Store error
    #[error("Store error: {0}")]
    StoreError(#[from] StoreError),

    /// Task execution failed
    #[error("Task execution failed: {0}")]
    TaskError(String),

    /// Internal error
    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Errors that can occur in the API layer
#[derive(Debug, Error)]
pub enum ApiError {
    /// Store error
    #[error("Store error: {0}")]
    StoreError(#[from] StoreError),

    /// Provider error
    #[error("Provider error: {0}")]
    ProviderError(#[from] ProviderError),

    /// Bad request
    #[error("Bad request: {0}")]
    BadRequest(String),

    /// Invalid request
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Unauthorized
    #[error("Unauthorized")]
    Unauthorized,

    /// Forbidden
    #[error("Forbidden")]
    Forbidden,

    /// Not found
    #[error("Not found")]
    NotFound,

    /// Rate limit exceeded
    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    /// Internal error
    #[error("Internal error: {0}")]
    InternalError(String),
}
