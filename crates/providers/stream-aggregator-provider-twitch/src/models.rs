//! Twitch-specific data models

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Configuration for the Twitch provider
#[derive(Debug, Clone)]
pub struct TwitchConfig {
    /// Twitch application client ID
    pub client_id: String,
    /// Twitch application client secret
    pub client_secret: String,
}

impl TwitchConfig {
    pub fn new(client_id: impl Into<String>, client_secret: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
        }
    }
}

/// Twitch-specific errors
#[derive(Debug, Error)]
pub enum TwitchError {
    #[error("HTTP request failed: {0}")]
    HttpError(String),

    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("Failed to parse response: {0}")]
    ParseError(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("User not found: {0}")]
    UserNotFound(String),
}

/// OAuth2 token response from Twitch
#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub token_type: String,
}

/// Twitch Helix API response for streams
#[derive(Debug, Deserialize)]
pub struct StreamsResponse {
    pub data: Vec<TwitchStream>,
    #[serde(default)]
    pub pagination: Pagination,
}

/// Twitch Helix API response for users
#[derive(Debug, Deserialize)]
pub struct UsersResponse {
    pub data: Vec<TwitchUser>,
}

/// Pagination cursor for Twitch API
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Pagination {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Stream data from Twitch Helix API
#[derive(Debug, Deserialize)]
pub struct TwitchStream {
    pub id: String,
    pub user_id: String,
    pub user_login: String,
    pub user_name: String,
    pub game_id: String,
    pub game_name: String,
    #[serde(rename = "type")]
    pub stream_type: String,
    pub title: String,
    pub viewer_count: u64,
    pub started_at: String,
    pub language: String,
    pub thumbnail_url: String,
    #[serde(default)]
    pub tag_ids: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub is_mature: bool,
}

/// User data from Twitch Helix API
#[derive(Debug, Deserialize)]
pub struct TwitchUser {
    pub id: String,
    pub login: String,
    pub display_name: String,
    #[serde(rename = "type")]
    pub user_type: String,
    pub broadcaster_type: String,
    pub description: String,
    pub profile_image_url: String,
    pub offline_image_url: String,
    pub view_count: u64,
    pub created_at: String,
}
