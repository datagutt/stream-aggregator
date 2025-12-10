//! Kick-specific data models

use serde::Deserialize;

/// Configuration for the Kick provider
#[derive(Debug, Clone)]
pub struct KickConfig {
    // Kick doesn't require API keys, just browser emulation
}

impl Default for KickConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl KickConfig {
    pub fn new() -> Self {
        Self {}
    }
}

/// Kick API channel response
#[derive(Debug, Deserialize)]
pub struct KickChannel {
    pub id: u64,
    pub user_id: u64,
    pub slug: String,
    pub user: KickUser,
    #[serde(default)]
    pub livestream: Option<KickLivestream>,
}

/// Kick user information
#[derive(Debug, Deserialize)]
pub struct KickUser {
    pub id: u64,
    pub username: String,
    #[serde(default)]
    pub profile_pic: Option<String>,
}

/// Kick livestream information
#[derive(Debug, Deserialize)]
pub struct KickLivestream {
    pub id: u64,
    pub slug: String,
    pub session_title: String,
    #[serde(default)]
    pub viewer_count: u64,
    pub created_at: String,
    pub language: String,
    #[serde(default)]
    pub thumbnail: Option<KickThumbnail>,
    #[serde(default)]
    pub categories: Vec<KickCategory>,
}

/// Kick thumbnail information
#[derive(Debug, Deserialize)]
pub struct KickThumbnail {
    #[serde(default)]
    pub url: Option<String>,
}

/// Kick category information
#[derive(Debug, Deserialize)]
pub struct KickCategory {
    pub id: u64,
    pub name: String,
    pub slug: String,
}
