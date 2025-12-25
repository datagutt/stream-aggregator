//! Trovo provider client implementation

use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;
use tracing::{debug, error};
use wreq::Client;

use stream_aggregator_core::{errors::ProviderError, models::*, traits::PlatformProvider};

use crate::models::{ChannelIdResponse, GetUsersResponse, TrovoConfig};

const TROVO_API_BASE: &str = "https://open-api.trovo.live/openplatform";

/// Trovo platform provider
pub struct TrovoProvider {
    client: Client,
    config: TrovoConfig,
}

impl TrovoProvider {
    /// Create a new Trovo provider
    pub fn new(config: TrovoConfig) -> Self {
        let client = Client::builder()
            .user_agent("StreamAggregator/1.0")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client, config }
    }

    /// Step 1: Fetch user by username to get channel_id
    async fn fetch_user_for_channel_id(&self, username: &str) -> Result<String, ProviderError> {
        debug!("Fetching Trovo user for channel_id: {}", username);

        let url = format!("{}/getusers", TROVO_API_BASE);
        let body = json!({
            "user": [username]
        });

        let response = self
            .client
            .post(&url)
            .header("Client-ID", &self.config.client_id)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::HttpError(format!("Failed to fetch Trovo user: {}", e)))?;

        let status = response.status();
        if status == 404 {
            return Err(ProviderError::StreamerNotFound(username.to_string()));
        }

        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            error!("Trovo API error {}: {}", status, body_text);
            return Err(ProviderError::HttpError(format!(
                "Trovo API error {}",
                status
            )));
        }

        let user_response: GetUsersResponse = response.json().await.map_err(|e| {
            ProviderError::ParseError(format!("Failed to parse Trovo user response: {}", e))
        })?;

        let user = user_response
            .users
            .into_iter()
            .next()
            .ok_or_else(|| ProviderError::StreamerNotFound(username.to_string()))?;

        Ok(user.channel_id)
    }

    /// Step 2: Fetch channel by channel_id
    async fn fetch_channel_by_id(
        &self,
        channel_id: &str,
    ) -> Result<ChannelIdResponse, ProviderError> {
        debug!("Fetching Trovo channel by ID: {}", channel_id);

        let url = format!("{}/channels/id", TROVO_API_BASE);
        let body = json!({
            "channel_id": channel_id
        });

        let response = self
            .client
            .post(&url)
            .header("Client-ID", &self.config.client_id)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                ProviderError::HttpError(format!("Failed to fetch Trovo channel: {}", e))
            })?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            error!("Trovo API error {}: {}", status, body_text);
            return Err(ProviderError::HttpError(format!(
                "Trovo API error {}",
                status
            )));
        }

        let channel_response: ChannelIdResponse = response.json().await.map_err(|e| {
            ProviderError::ParseError(format!("Failed to parse Trovo channel response: {}", e))
        })?;

        Ok(channel_response)
    }
}

#[async_trait]
impl PlatformProvider for TrovoProvider {
    fn platform_id(&self) -> &'static str {
        "trovo"
    }

    fn display_name(&self) -> &'static str {
        "Trovo"
    }

    fn base_url(&self) -> &'static str {
        "https://trovo.live"
    }

    async fn resolve_user_id(&self, username_or_id: &str) -> Result<String, ProviderError> {
        // Trovo uses usernames
        Ok(username_or_id.to_string())
    }

    async fn fetch_stream(&self, user_id: &str) -> Result<StreamInfo, ProviderError> {
        // Two-step process: getusers -> channels/id
        let channel_id = self.fetch_user_for_channel_id(user_id).await?;
        let channel = self.fetch_channel_by_id(&channel_id).await?;

        let mut stream_info = StreamInfo::new("trovo", &channel.username, &channel.nickname);
        stream_info.avatar_url = Some(channel.profile_pic);
        stream_info.is_live = channel.is_live;

        if channel.is_live {
            stream_info.title = Some(channel.live_title);
            stream_info.viewer_count = Some(channel.current_viewers);
            stream_info.category = Some(channel.category_name);
            stream_info.language = channel.language_code;
        }

        stream_info.last_updated = Utc::now();

        Ok(stream_info)
    }

    async fn fetch_streams_batch(
        &self,
        user_ids: &[String],
    ) -> Vec<Result<StreamInfo, ProviderError>> {
        // Trovo requires two-step lookup per user (no batch support for channels/id)
        let mut results = Vec::with_capacity(user_ids.len());

        for user_id in user_ids {
            results.push(self.fetch_stream(user_id).await);
        }

        results
    }

    fn supports_discovery(&self) -> bool {
        false
    }

    async fn discover_streamers(
        &self,
        _filters: &DiscoveryFilters,
    ) -> Result<Vec<DiscoveredStreamer>, ProviderError> {
        Err(ProviderError::DiscoveryNotSupported)
    }

    fn rate_limit_config(&self) -> RateLimitConfig {
        RateLimitConfig {
            requests_per_minute: 120,
            burst_size: 20,
        }
    }

    async fn health_check(&self) -> HealthStatus {
        match self.client.get("https://trovo.live").send().await {
            Ok(response) if response.status().is_success() => HealthStatus::Healthy,
            _ => HealthStatus::Unhealthy,
        }
    }
}
