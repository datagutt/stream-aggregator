//! AngelThump provider client implementation

use async_trait::async_trait;
use chrono::Utc;
use tracing::debug;
use wreq::Client;

use stream_aggregator_core::{errors::ProviderError, models::*, traits::PlatformProvider};

use crate::models::{AngelThumpConfig, AngelThumpStream, AngelThumpUser};

/// AngelThump platform provider (two-endpoint lookup)
pub struct AngelThumpProvider {
    client: Client,
}

impl AngelThumpProvider {
    /// Create a new AngelThump provider
    pub fn new(_config: AngelThumpConfig) -> Self {
        let client = Client::builder()
            .user_agent("StreamAggregator/1.0")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client }
    }

    /// Fetch user info from AngelThump API (uses query params, returns array)
    async fn fetch_user(&self, username: &str) -> Result<AngelThumpUser, ProviderError> {
        debug!("Fetching AngelThump user: {}", username);

        let url = format!("https://api.angelthump.com/v3/users/?username={}", username);

        let response = self.client.get(&url).send().await.map_err(|e| {
            ProviderError::HttpError(format!("Failed to fetch AngelThump user: {}", e))
        })?;

        let status = response.status();
        if status == 404 {
            return Err(ProviderError::StreamerNotFound(username.to_string()));
        }

        if !status.is_success() {
            return Err(ProviderError::HttpError(format!(
                "AngelThump API error {}",
                status
            )));
        }

        // API returns an array, take first element
        let users: Vec<AngelThumpUser> = response.json().await.map_err(|e| {
            ProviderError::ParseError(format!("Failed to parse AngelThump user response: {}", e))
        })?;

        users
            .into_iter()
            .next()
            .ok_or_else(|| ProviderError::StreamerNotFound(username.to_string()))
    }

    /// Fetch stream info from AngelThump API (uses query params, returns array)
    async fn fetch_stream_info(
        &self,
        username: &str,
    ) -> Result<Option<AngelThumpStream>, ProviderError> {
        debug!("Fetching AngelThump stream: {}", username);

        let url = format!(
            "https://api.angelthump.com/v3/streams/?username={}",
            username
        );

        let response = self.client.get(&url).send().await.map_err(|e| {
            ProviderError::HttpError(format!("Failed to fetch AngelThump stream: {}", e))
        })?;

        let status = response.status();
        if status == 404 {
            // Stream not live
            return Ok(None);
        }

        if !status.is_success() {
            return Ok(None); // Gracefully handle errors for stream info
        }

        // API returns an array
        let streams: Vec<AngelThumpStream> = response.json().await.map_err(|_| {
            ProviderError::ParseError("Failed to parse stream response".to_string())
        })?;

        Ok(streams.into_iter().next())
    }
}

#[async_trait]
impl PlatformProvider for AngelThumpProvider {
    fn platform_id(&self) -> &'static str {
        "angelthump"
    }

    fn display_name(&self) -> &'static str {
        "AngelThump"
    }

    fn base_url(&self) -> &'static str {
        "https://angelthump.com"
    }

    async fn resolve_user_id(&self, username_or_id: &str) -> Result<String, ProviderError> {
        Ok(username_or_id.to_string())
    }

    async fn fetch_stream(&self, user_id: &str) -> Result<StreamInfo, ProviderError> {
        let user = self.fetch_user(user_id).await?;
        let stream = self.fetch_stream_info(user_id).await.ok().flatten();

        let is_live = stream.is_some();
        let viewer_count = stream.as_ref().and_then(|s| s.viewer_count);

        let mut stream_info = StreamInfo::new("angelthump", user_id, &user.username);
        stream_info.is_live = is_live;
        stream_info.title = user.title;
        stream_info.thumbnail_url = user.thumbnail;
        stream_info.viewer_count = viewer_count;
        stream_info.last_updated = Utc::now();

        Ok(stream_info)
    }

    async fn fetch_streams_batch(
        &self,
        user_ids: &[String],
    ) -> Vec<Result<StreamInfo, ProviderError>> {
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
            requests_per_minute: 60,
            burst_size: 10,
        }
    }

    async fn health_check(&self) -> HealthStatus {
        match self.client.get("https://angelthump.com").send().await {
            Ok(response) if response.status().is_success() => HealthStatus::Healthy,
            _ => HealthStatus::Unhealthy,
        }
    }
}
