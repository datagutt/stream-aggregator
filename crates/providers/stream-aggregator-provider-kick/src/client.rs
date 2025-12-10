//! Kick provider client implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, warn};
use wreq::Client;
use wreq_util::Emulation;

use stream_aggregator_core::{
    errors::ProviderError,
    models::*,
    traits::PlatformProvider,
};

use crate::models::{KickConfig, KickChannel};

const KICK_API_BASE: &str = "https://kick.com/api";

/// Kick platform provider with Cloudflare bypass
pub struct KickProvider {
    /// wreq client with Chrome browser emulation for Cloudflare bypass
    client: Client,
    #[allow(dead_code)]
    config: KickConfig,
    /// Cached XSRF token
    xsrf_token: Arc<RwLock<Option<String>>>,
}

impl KickProvider {
    /// Create a new Kick provider with browser emulation
    pub fn new(config: KickConfig) -> Result<Self, ProviderError> {
        // Create wreq client with Chrome 131 emulation to bypass Cloudflare
let client = Client::builder()
            .emulation(Emulation::Chrome131)
            .build()
            .map_err(|e| ProviderError::InitializationError(format!("Failed to create Kick client: {}", e)))?;

        Ok(Self {
            client,
            config,
            xsrf_token: Arc::new(RwLock::new(None)),
        })
    }

    /// Ensure we have a valid XSRF token
    async fn ensure_xsrf_token(&self) -> Result<(), ProviderError> {
        let token = self.xsrf_token.read().await;
        if token.is_some() {
            return Ok(());
        }
        drop(token);

        // Need to fetch XSRF token
        debug!("Fetching Kick XSRF token");
        
        let response = self.client
            .get("https://kick.com")
            .send()
            .await
            .map_err(|e| ProviderError::HttpError(format!("Failed to fetch Kick homepage: {}", e)))?;

        // Extract XSRF token from cookies
        let mut token_value = None;
        for cookie in response.cookies() {
            if cookie.name() == "XSRF-TOKEN" {
                token_value = Some(cookie.value().to_string());
                break;
            }
        }

        if let Some(token) = token_value {
            let mut xsrf = self.xsrf_token.write().await;
            *xsrf = Some(token);
            debug!("XSRF token obtained");
            Ok(())
        } else {
            warn!("No XSRF token found in response");
            Ok(()) // Continue without token, some endpoints might work
        }
    }

    /// Get the current XSRF token
    async fn get_xsrf_token(&self) -> Option<String> {
        self.xsrf_token.read().await.clone()
    }

    /// Fetch channel information from Kick API
    async fn fetch_channel(&self, username: &str) -> Result<KickChannel, ProviderError> {
        debug!("Fetching Kick channel: {}", username);

        // Ensure we have XSRF token
        self.ensure_xsrf_token().await?;

        let url = format!("{}/v2/channels/{}", KICK_API_BASE, username);
        
        let mut request = self.client.get(&url);
        
        // Add XSRF token if we have one
        if let Some(token) = self.get_xsrf_token().await {
            request = request.header("X-XSRF-TOKEN", token);
        }

        let response = request
            .send()
            .await
            .map_err(|e| ProviderError::HttpError(format!("Failed to fetch Kick channel: {}", e)))?;

        let status = response.status();
        
        if status == 403 || status == 429 {
            // Rate limited or blocked by Cloudflare
            warn!("Kick API returned {}, possible rate limit or Cloudflare block", status);
            return Err(ProviderError::RateLimited);
        }

        if status == 404 {
            return Err(ProviderError::StreamerNotFound(username.to_string()));
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            error!("Kick API error {}: {}", status, body);
            return Err(ProviderError::HttpError(format!("Kick API error {}: {}", status, body)));
        }

        let channel: KickChannel = response
            .json()
            .await
            .map_err(|e| ProviderError::ParseError(format!("Failed to parse Kick channel response: {}", e)))?;

        Ok(channel)
    }
}

#[async_trait]
impl PlatformProvider for KickProvider {
    fn platform_id(&self) -> &'static str {
        "kick"
    }

    fn display_name(&self) -> &'static str {
        "Kick"
    }

    fn base_url(&self) -> &'static str {
        "https://kick.com"
    }

    async fn resolve_user_id(&self, username_or_id: &str) -> Result<String, ProviderError> {
        // Kick uses usernames (slugs) as identifiers
        Ok(username_or_id.to_string())
    }

    async fn fetch_stream(&self, user_id: &str) -> Result<StreamInfo, ProviderError> {
        let channel = self.fetch_channel(user_id).await?;

        let mut stream_info = StreamInfo::new("kick", &channel.slug, &channel.user.username);
        
        // Set avatar
        if let Some(profile_pic) = channel.user.profile_pic {
            stream_info.avatar_url = Some(profile_pic);
        }

        // Check if live and populate stream info
        if let Some(livestream) = channel.livestream {
            stream_info.is_live = true;
            stream_info.title = Some(livestream.session_title);
            stream_info.viewer_count = Some(livestream.viewer_count);
            stream_info.language = Some(livestream.language);

            // Parse started_at
            if let Ok(started_at) = livestream.created_at.parse::<DateTime<Utc>>() {
                stream_info.started_at = Some(started_at);
            }

            // Set thumbnail
            if let Some(thumbnail) = livestream.thumbnail {
                if let Some(url) = thumbnail.url {
                    stream_info.thumbnail_url = Some(url);
                }
            }

            // Set category
            if let Some(category) = livestream.categories.first() {
                stream_info.category = Some(category.name.clone());
            }
        } else {
            stream_info.is_live = false;
        }

        stream_info.last_updated = Utc::now();

        Ok(stream_info)
    }

    async fn fetch_streams_batch(&self, user_ids: &[String]) -> Vec<Result<StreamInfo, ProviderError>> {
        // Kick doesn't have a batch API, fetch sequentially with delays
        let mut results = Vec::with_capacity(user_ids.len());
        
        for user_id in user_ids {
            results.push(self.fetch_stream(user_id).await);
            
            // Add delay to avoid rate limiting
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
        
        results
    }

    fn supports_discovery(&self) -> bool {
        false // Discovery not implemented yet
    }

    async fn discover_streamers(
        &self,
        _filters: &DiscoveryFilters,
    ) -> Result<Vec<DiscoveredStreamer>, ProviderError> {
        Err(ProviderError::DiscoveryNotSupported)
    }

    fn rate_limit_config(&self) -> RateLimitConfig {
        RateLimitConfig {
            requests_per_minute: 30, // Conservative for Cloudflare
            burst_size: 5,
        }
    }

    async fn health_check(&self) -> HealthStatus {
        match self.client.get("https://kick.com").send().await {
            Ok(response) if response.status().is_success() => HealthStatus::Healthy,
            _ => HealthStatus::Unhealthy,
        }
    }
}
