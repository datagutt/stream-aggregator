//! DLive-specific data models

use serde::{Deserialize, Serialize};

/// Configuration for the DLive provider
#[derive(Debug, Clone)]
pub struct DLiveConfig {
    // No API key required
}

impl Default for DLiveConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl DLiveConfig {
    pub fn new() -> Self {
        Self {}
    }
}

/// DLive GraphQL response
#[derive(Debug, Deserialize)]
pub struct GraphQLResponse {
    pub data: Option<UserByDisplayNameData>,
    #[serde(default)]
    pub errors: Vec<GraphQLError>,
}

/// GraphQL error
#[derive(Debug, Deserialize)]
pub struct GraphQLError {
    pub message: String,
}

/// User by display name data
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserByDisplayNameData {
    pub user_by_display_name: Option<DLiveUser>,
}

/// DLive user information - only essential fields
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DLiveUser {
    pub id: String,
    pub avatar: String,
    pub displayname: String,
    pub username: String,
    pub livestream: Option<DLiveLivestream>,
}

/// DLive livestream information
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DLiveLivestream {
    pub id: String,
    pub title: String,
    pub watching_count: u64,
    pub language: Option<DLiveLanguage>,
    pub category: Option<DLiveCategory>,
}

/// DLive language
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DLiveLanguage {
    pub language: String,
}

/// DLive category
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DLiveCategory {
    pub title: String,
}
