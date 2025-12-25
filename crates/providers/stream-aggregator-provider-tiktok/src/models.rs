//! TikTok provider models

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TikTokConfig {
    /// URL of the TikTok bridge HTTP server
    #[serde(default = "default_bridge_url")]
    pub bridge_url: String,
    /// Path to the Node.js bridge directory
    pub bridge_path: Option<String>,
    /// Maximum concurrent requests to the bridge
    pub max_concurrent_requests: Option<usize>,
    /// Request timeout in milliseconds
    pub request_timeout_ms: Option<u64>,
    /// Batch size for multi-user queries
    pub batch_size: Option<usize>,
    /// Requests per minute limit
    pub requests_per_minute: Option<u32>,
    /// Burst size for rate limiting
    pub burst_size: Option<u32>,
}

fn default_bridge_url() -> String {
    "http://127.0.0.1:3456".to_string()
}

impl Default for TikTokConfig {
    fn default() -> Self {
        Self {
            bridge_url: default_bridge_url(),
            bridge_path: None,
            max_concurrent_requests: None,
            request_timeout_ms: None,
            batch_size: None,
            requests_per_minute: None,
            burst_size: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub uptime: u64,
    #[serde(rename = "activeConnections")]
    pub active_connections: usize,
    #[serde(rename = "totalRequests")]
    pub total_requests: u64,
    #[serde(rename = "cacheSize")]
    pub cache_size: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomInfoRequest {
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchRoomInfoRequest {
    pub usernames: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInfoResponse {
    pub success: bool,
    pub data: Option<TikTokStreamData>,
    pub error: Option<String>,
    pub error_code: Option<String>,
    pub retry_after: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRoomInfoResponse {
    pub success: bool,
    pub results: Vec<BatchResultItem>,
    pub stats: Option<BatchStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResultItem {
    pub username: String,
    pub success: bool,
    pub data: Option<TikTokStreamData>,
    pub error: Option<String>,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchStats {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TikTokStreamData {
    pub live: bool,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub viewer_count: Option<u64>,
    pub title: Option<String>,
    pub room_id: Option<String>,
    pub stream_url: Option<String>,
    pub bio: Option<String>,
    pub create_time: Option<u64>,
}

impl TikTokStreamData {
    #[allow(dead_code)]
    pub fn offline(username: &str) -> Self {
        Self {
            live: false,
            username: username.to_string(),
            display_name: username.to_string(),
            avatar_url: None,
            thumbnail_url: None,
            viewer_count: None,
            title: None,
            room_id: None,
            stream_url: None,
            bio: None,
            create_time: None,
        }
    }
}

/// Known error codes from the bridge
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TikTokErrorCode {
    Unknown,
    UserNotFound,
    UserOffline,
    RateLimited,
    InvalidResponse,
    Timeout,
    NetworkError,
    CaptchaRequired,
}

impl From<Option<&str>> for TikTokErrorCode {
    fn from(code: Option<&str>) -> Self {
        match code {
            Some("user_not_found") => Self::UserNotFound,
            Some("user_offline") => Self::UserOffline,
            Some("rate_limited") => Self::RateLimited,
            Some("invalid_response") => Self::InvalidResponse,
            Some("timeout") => Self::Timeout,
            Some("network_error") => Self::NetworkError,
            Some("captcha_required") => Self::CaptchaRequired,
            _ => Self::Unknown,
        }
    }
}
