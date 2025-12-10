//! YouTube scraper implementation (matching lsnd youtube.js)

use async_trait::async_trait;
use chrono::Utc;
use regex::Regex;
use serde_json::Value;
use std::sync::OnceLock;
use tracing::debug;

use stream_aggregator_core::{errors::ProviderError, models::*, traits::PlatformProvider};
use wreq::{redirect::Policy, Client};
use wreq_util::Emulation;

use crate::models::YouTubeConfig;

// Compiled regexes for efficient reuse
static AVATAR_REGEX: OnceLock<Regex> = OnceLock::new();
static NAME_REGEX: OnceLock<Regex> = OnceLock::new();
static TITLE_REGEX: OnceLock<Regex> = OnceLock::new();
static STATUS_REGEX: OnceLock<Regex> = OnceLock::new();
static VIEWERS_REGEX: OnceLock<Regex> = OnceLock::new();

/// YouTube platform provider using pure HTML scraping (matching lsnd)
pub struct YouTubeProvider {
    client: Client,
}

impl YouTubeProvider {
    /// Create a new YouTube provider
    pub fn new(_config: YouTubeConfig) -> Self {
        let client = Client::builder()
            .emulation(Emulation::Chrome131)
            .redirect(Policy::default())
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client }
    }

    fn avatar_regex() -> &'static Regex {
        AVATAR_REGEX.get_or_init(|| {
            Regex::new(r#"(?s)channelMetadataRenderer.*?avatar.*?thumbnails.*?url".*?"(.*?)""#)
                .unwrap()
        })
    }

    fn name_regex() -> &'static Regex {
        NAME_REGEX.get_or_init(|| {
            Regex::new(r#"(?s)channelMetadataRenderer.*?title".*?"(.*?)""#).unwrap()
        })
    }

    fn title_regex() -> &'static Regex {
        TITLE_REGEX.get_or_init(|| {
            Regex::new(r#"(?s)videoPrimaryInfoRenderer".*?"title".*?"runs".*?"text".*?"(.*?)""#)
                .unwrap()
        })
    }

    fn status_regex() -> &'static Regex {
        STATUS_REGEX
            .get_or_init(|| Regex::new(r#"(?s)playabilityStatus.*?status".*?"(.*?)""#).unwrap())
    }

    fn viewers_regex() -> &'static Regex {
        VIEWERS_REGEX.get_or_init(|| {
            Regex::new(r#"(?s)videoPrimaryInfoRenderer".*?"viewCount".*?"runs".*?"text".*?"(.*?)""#)
                .unwrap()
        })
    }
}

#[async_trait]
impl PlatformProvider for YouTubeProvider {
    fn platform_id(&self) -> &'static str {
        "youtube"
    }

    fn display_name(&self) -> &'static str {
        "YouTube"
    }

    fn base_url(&self) -> &'static str {
        "https://youtube.com"
    }

    async fn resolve_user_id(&self, username_or_id: &str) -> Result<String, ProviderError> {
        Ok(username_or_id.to_string())
    }

    async fn fetch_stream(&self, user_id: &str) -> Result<StreamInfo, ProviderError> {
        debug!("Fetching YouTube channel: {}", user_id);

        // Fetch channel page for metadata
        let channel_url = format!("https://www.youtube.com/channel/{}/featured", user_id);
        let data = self
            .client
            .get(&channel_url)
            .send()
            .await
            .map_err(|e| {
                ProviderError::HttpError(format!("Failed to fetch YouTube channel: {}", e))
            })?
            .text()
            .await
            .map_err(|e| ProviderError::HttpError(format!("Failed to read response: {}", e)))?;

        // Extract avatar and name
        let avatar = Self::avatar_regex()
            .captures(&data)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string());

        let name = Self::name_regex()
            .captures(&data)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| ProviderError::ParseError("Could not find channel name".to_string()))?;

        // Fetch live page
        let live_url = format!("https://www.youtube.com/channel/{}/live", user_id);
        let livedata = self
            .client
            .get(&live_url)
            .send()
            .await
            .map_err(|e| {
                ProviderError::HttpError(format!("Failed to fetch YouTube live page: {}", e))
            })?
            .text()
            .await
            .map_err(|e| ProviderError::HttpError(format!("Failed to read response: {}", e)))?;

        // Check if live
        let mut live = false;
        let mut title = None;
        let mut viewers = None;

        if let Some(title_match) = Self::title_regex()
            .captures(&livedata)
            .and_then(|c| c.get(1))
        {
            if let Some(status_match) = Self::status_regex()
                .captures(&livedata)
                .and_then(|c| c.get(1))
            {
                live = status_match.as_str() != "LIVE_STREAM_OFFLINE";
            }
            title = Some(title_match.as_str().to_string());
        }

        // Extract viewer count if live
        if live {
            if let Some(viewers_match) = Self::viewers_regex()
                .captures(&livedata)
                .and_then(|c| c.get(1))
            {
                let viewers_str = viewers_match.as_str();
                // Extract first part before space and remove commas/periods
                if let Some(first_part) = viewers_str.split(' ').next() {
                    let cleaned = first_part.replace(&[',', '.'][..], "");
                    viewers = cleaned.parse::<u64>().ok();
                }
            }
        }

        // Build stream info
        let mut stream_info = StreamInfo::new("youtube", user_id, &name);
        stream_info.avatar_url = avatar;
        stream_info.is_live = live;
        stream_info.title = title;
        stream_info.viewer_count = viewers;
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
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
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
        match self.client.get("https://www.youtube.com").send().await {
            Ok(response) if response.status().is_success() => HealthStatus::Healthy,
            _ => HealthStatus::Unhealthy,
        }
    }
}
