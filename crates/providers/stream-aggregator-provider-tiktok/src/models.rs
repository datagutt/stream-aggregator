//! TikTok provider models

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TikTokConfig {
	pub node_path: Option<String>,
	pub bridge_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BridgeCommand {
	pub action: String,
	pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BridgeResponse {
	pub success: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub data: Option<TikTokStreamData>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub error: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub pong: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TikTokStreamData {
	pub live: bool,
	pub name: String,
	pub avatar: String,
	pub thumbnail_url: Option<String>,
	pub viewers: Option<u64>,
	pub title: Option<String>,
}
