//! Trovo-specific data models

use serde::Deserialize;

/// Configuration for the Trovo provider
#[derive(Debug, Clone)]
pub struct TrovoConfig {
    pub client_id: String,
}

impl Default for TrovoConfig {
    fn default() -> Self {
        Self {
            client_id: String::new(),
        }
    }
}

impl TrovoConfig {
    pub fn new(client_id: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
        }
    }
}

/// Trovo getusers response
#[derive(Debug, Deserialize)]
pub struct GetUsersResponse {
    pub users: Vec<TrovoUser>,
}

/// Trovo user information from getusers
#[derive(Debug, Deserialize)]
pub struct TrovoUser {
    pub user_id: Option<String>,
    pub username: String,
    pub nickname: String,
    pub profile_pic: String,
    pub channel_id: String,
}

/// Trovo channels/id response
#[derive(Debug, Deserialize)]
pub struct ChannelIdResponse {
    pub is_live: bool,
    pub category_name: String,
    pub live_title: String,
    pub current_viewers: u64,
    pub username: String,
    pub nickname: String,
    pub profile_pic: String,
    #[serde(default)]
    pub language_code: Option<String>,
}
