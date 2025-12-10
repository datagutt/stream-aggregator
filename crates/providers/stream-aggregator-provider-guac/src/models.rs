//! Guac-specific data models

use serde::Deserialize;

/// Configuration for the Guac provider
#[derive(Debug, Clone)]
pub struct GuacConfig {}

impl Default for GuacConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl GuacConfig {
    pub fn new() -> Self {
        Self {}
    }
}

/// Guac API response wrapper
#[derive(Debug, Deserialize)]
pub struct GuacResponse {
    pub data: GuacStreamData,
}

/// Guac stream data (nested inside response.data)
#[derive(Debug, Deserialize)]
pub struct GuacStreamData {
    pub live: bool,
    pub user: GuacUser,
    #[serde(rename = "type")]
    #[serde(default)]
    pub stream_type: Option<String>,
    #[serde(default)]
    pub viewers: Option<u64>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub banner: Option<String>,
}

/// Guac user information
#[derive(Debug, Deserialize)]
pub struct GuacUser {
    pub username: String,
    #[serde(default)]
    pub avatar: Option<String>,
}
