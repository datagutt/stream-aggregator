//! AngelThump-specific data models

use serde::Deserialize;

/// Configuration for the AngelThump provider
#[derive(Debug, Clone)]
pub struct AngelThumpConfig {}

impl Default for AngelThumpConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl AngelThumpConfig {
    pub fn new() -> Self {
        Self {}
    }
}

/// AngelThump user response (API returns an array)
#[derive(Debug, Deserialize)]
pub struct AngelThumpUser {
    pub username: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub thumbnail: Option<String>,
}

/// AngelThump stream response (API returns an array).
///
/// `createdAt` marks when the stream went live on the platform and is the
/// source for `StreamInfo::started_at`. `updatedAt` is the platform's own
/// heartbeat and is intentionally not consumed: our `last_fetched_at` covers
/// recency from our side.
#[derive(Debug, Deserialize)]
pub struct AngelThumpStream {
    pub username: String,
    #[serde(default)]
    pub viewer_count: Option<u64>,
    #[serde(default, rename = "createdAt")]
    pub created_at: Option<String>,
}
