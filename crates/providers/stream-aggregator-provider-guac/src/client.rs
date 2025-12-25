//! Guac provider client implementation

use async_trait::async_trait;
use chrono::Utc;
use tracing::debug;
use wreq::Client;

use stream_aggregator_core::{errors::ProviderError, models::*, traits::PlatformProvider};

use crate::models::{GuacConfig, GuacResponse};

/// Guac platform provider
pub struct GuacProvider {
    client: Client,
}

impl GuacProvider {
    /// Create a new Guac provider
    pub fn new(_config: GuacConfig) -> Self {
        let client = Client::builder()
            .user_agent("StreamAggregator/1.0")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client }
    }

    /// Fetch stream info from Guac API
    async fn fetch_guac_stream(&self, user_id: &str) -> Result<GuacResponse, ProviderError> {
        debug!("Fetching Guac stream: {}", user_id);

        // Guac uses api.guac.tv, not guac.live for API calls
        let url = format!("https://api.guac.tv/v2/stream/{}", user_id);

        let response =
            self.client.get(&url).send().await.map_err(|e| {
                ProviderError::HttpError(format!("Failed to fetch Guac stream: {}", e))
            })?;

        let status = response.status();
        if status == 404 {
            return Err(ProviderError::StreamerNotFound(user_id.to_string()));
        }

        if !status.is_success() {
            return Err(ProviderError::HttpError(format!(
                "Guac API error {}",
                status
            )));
        }

        let api_response: GuacResponse = response.json().await.map_err(|e| {
            ProviderError::ParseError(format!("Failed to parse Guac response: {}", e))
        })?;

        Ok(api_response)
    }
}

#[async_trait]
impl PlatformProvider for GuacProvider {
    fn platform_id(&self) -> &'static str {
        "guac"
    }

    fn display_name(&self) -> &'static str {
        "Guac"
    }

    fn base_url(&self) -> &'static str {
        "https://guac.tv"
    }

    async fn resolve_user_id(&self, username_or_id: &str) -> Result<String, ProviderError> {
        Ok(username_or_id.to_string())
    }

    async fn fetch_stream(&self, user_id: &str) -> Result<StreamInfo, ProviderError> {
        let api_response = self.fetch_guac_stream(user_id).await?;
        let stream = api_response.data;

        let mut stream_info = StreamInfo::new("guac", user_id, &stream.user.username);
        stream_info.is_live = stream.live;
        stream_info.title = stream.title;
        stream_info.viewer_count = stream.viewers;
        stream_info.thumbnail_url = stream.banner; // Use banner as thumbnail
        stream_info.avatar_url = stream.user.avatar;
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
        match self.client.get("https://api.guac.tv").send().await {
            Ok(response) if response.status().is_success() => HealthStatus::Healthy,
            _ => HealthStatus::Unhealthy,
        }
    }
}
