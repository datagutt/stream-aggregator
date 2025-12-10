//! Twitch provider client implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tracing::{debug, error, warn};
use wreq::Client;

use stream_aggregator_core::{
    errors::ProviderError,
    models::*,
    traits::PlatformProvider,
};

use crate::auth::TokenManager;
use crate::models::{TwitchConfig, TwitchError, StreamsResponse, UsersResponse};

const HELIX_API_BASE: &str = "https://api.twitch.tv/helix";

/// Twitch platform provider
pub struct TwitchProvider {
    client: Client,
    config: TwitchConfig,
    token_manager: TokenManager,
}

impl TwitchProvider {
    /// Create a new Twitch provider
    pub fn new(config: TwitchConfig) -> Self {
        let client = Client::new();
        let token_manager = TokenManager::new(client.clone(), config.clone());

        Self {
            client,
            config,
            token_manager,
        }
    }

    /// Get headers for authenticated requests
    async fn auth_headers(&self) -> Result<HashMap<&'static str, String>, ProviderError> {
        let token = self.token_manager.get_token().await.map_err(|e| {
            ProviderError::AuthError(format!("Failed to get Twitch token: {}", e))
        })?;

        let mut headers = HashMap::new();
        headers.insert("Authorization", format!("Bearer {}", token));
        headers.insert("Client-Id", self.config.client_id.clone());
        Ok(headers)
    }

    /// Resolve a username (login) to a user ID
    pub async fn resolve_username_to_user_id(&self, username: &str) -> Result<String, ProviderError> {
        let headers = self.auth_headers().await?;

        let url = format!("{}/users", HELIX_API_BASE);
        let mut request = self.client.get(&url);

        request = request.query(&[("login", username)]);

        for (key, value) in headers {
            request = request.header(key, value);
        }

        let response = request.send().await.map_err(|e| {
            ProviderError::HttpError(format!("Failed to fetch Twitch user: {}", e))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::HttpError(format!(
                "Twitch API error {}: {}",
                status, body
            )));
        }

        let users_response: UsersResponse = response.json().await.map_err(|e| {
            ProviderError::ParseError(format!("Failed to parse Twitch users response: {}", e))
        })?;

        if users_response.data.is_empty() {
            return Err(ProviderError::StreamerNotFound(username.to_string()));
        }

        Ok(users_response.data[0].id.clone())
    }



    /// Fetch stream information by user ID
    async fn fetch_stream_by_user_id(&self, user_id: &str) -> Result<StreamInfo, ProviderError> {
        let headers = self.auth_headers().await?;

        // First, get user info
        let user_url = format!("{}/users?id={}", HELIX_API_BASE, user_id);
        let mut user_request = self.client.get(&user_url);
        for (key, value) in &headers {
            user_request = user_request.header(*key, value);
        }

        let user_response = user_request.send().await.map_err(|e| {
            ProviderError::HttpError(format!("Failed to fetch Twitch user info: {}", e))
        })?;

        let users: UsersResponse = user_response.json().await.map_err(|e| {
            ProviderError::ParseError(format!("Failed to parse Twitch user response: {}", e))
        })?;

        if users.data.is_empty() {
            return Err(ProviderError::StreamerNotFound(user_id.to_string()));
        }

        let user = &users.data[0];

        // Then check if they're streaming
        let stream_url = format!("{}/streams?user_id={}", HELIX_API_BASE, user_id);
        let mut stream_request = self.client.get(&stream_url);
        for (key, value) in &headers {
            stream_request = stream_request.header(*key, value);
        }

        let stream_response = stream_request.send().await.map_err(|e| {
            ProviderError::HttpError(format!("Failed to fetch Twitch stream: {}", e))
        })?;

        let streams: StreamsResponse = stream_response.json().await.map_err(|e| {
            ProviderError::ParseError(format!("Failed to parse Twitch streams response: {}", e))
        })?;

        let mut stream_info = StreamInfo::new("twitch", &user.id, &user.display_name);
        stream_info.avatar_url = Some(user.profile_image_url.clone());

        if let Some(stream) = streams.data.first() {
            // User is live
            stream_info.is_live = true;
            stream_info.title = Some(stream.title.clone());
            stream_info.viewer_count = Some(stream.viewer_count);
            stream_info.category = Some(stream.game_name.clone());
            stream_info.language = Some(stream.language.clone());
            stream_info.tags = stream.tags.clone();

            // Parse started_at timestamp
            if let Ok(started_at) = stream.started_at.parse::<DateTime<Utc>>() {
                stream_info.started_at = Some(started_at);
            }

            // Build thumbnail URL (replace template variables)
            let thumbnail = stream.thumbnail_url
                .replace("{width}", "1280")
                .replace("{height}", "720");
            stream_info.thumbnail_url = Some(thumbnail);
        } else {
            // User is offline
            stream_info.is_live = false;
        }

        stream_info.last_updated = Utc::now();

        Ok(stream_info)
    }
}

#[async_trait]
impl PlatformProvider for TwitchProvider {
    fn platform_id(&self) -> &'static str {
        "twitch"
    }

    fn display_name(&self) -> &'static str {
        "Twitch"
    }

    fn base_url(&self) -> &'static str {
        "https://twitch.tv"
    }

    async fn resolve_user_id(&self, username_or_id: &str) -> Result<String, ProviderError> {
        if username_or_id.chars().all(|c| c.is_ascii_digit()) {
            return Ok(username_or_id.to_string());
        }
        
        self.resolve_username_to_user_id(username_or_id).await
    }

    async fn fetch_stream(&self, user_id: &str) -> Result<StreamInfo, ProviderError> {
        self.fetch_stream_by_user_id(user_id).await
    }

    async fn fetch_streams_batch(&self, user_ids: &[String]) -> Vec<Result<StreamInfo, ProviderError>> {
        if user_ids.is_empty() {
            return Vec::new();
        }

        debug!(count = user_ids.len(), "Batch fetching Twitch streams");

        // Twitch supports up to 100 user IDs per request
        let chunk_size = 100;
        let mut results = Vec::with_capacity(user_ids.len());

        for chunk in user_ids.chunks(chunk_size) {
            let headers = match self.auth_headers().await {
                Ok(h) => h,
                Err(e) => {
                    // If auth fails, return error for all in this chunk
                    for _ in chunk {
                        results.push(Err(e.clone()));
                    }
                    continue;
                }
            };

            // Build query string with multiple user ID parameters
            let url = format!("{}/users", HELIX_API_BASE);
            let mut request = self.client.get(&url);

            for user_id in chunk {
                request = request.query(&[("id", user_id)]);
            }

            for (key, value) in &headers {
                request = request.header(*key, value);
            }

            let response = match request.send().await {
                Ok(r) => r,
                Err(e) => {
                    let err = ProviderError::HttpError(format!("Batch request failed: {}", e));
                    for _ in chunk {
                        results.push(Err(err.clone()));
                    }
                    continue;
                }
            };

            let users: UsersResponse = match response.json().await {
                Ok(u) => u,
                Err(e) => {
                    let err = ProviderError::ParseError(format!("Failed to parse batch response: {}", e));
                    for _ in chunk {
                        results.push(Err(err.clone()));
                    }
                    continue;
                }
            };

            // Create a map of user_id -> user for quick lookup
            let user_map: HashMap<_, _> = users
                .data
                .into_iter()
                .map(|u| (u.id.clone(), u))
                .collect();

            // Fetch stream status for all found users
            for user_id in chunk {
                if let Some(user) = user_map.get(user_id) {
                    match self.fetch_stream_by_user_id(&user.id).await {
                        Ok(info) => results.push(Ok(info)),
                        Err(e) => results.push(Err(e)),
                    }
                } else {
                    results.push(Err(ProviderError::StreamerNotFound(user_id.to_string())));
                }
            }
        }

        results
    }

    fn supports_discovery(&self) -> bool {
        true
    }

    async fn discover_streamers(
        &self,
        filters: &DiscoveryFilters,
    ) -> Result<Vec<DiscoveredStreamer>, ProviderError> {
        debug!(?filters, "Discovering Twitch streamers");

        let headers = self.auth_headers().await?;
        let limit = filters.limit.unwrap_or(20).min(100);

        let url = format!("{}/streams", HELIX_API_BASE);
        let mut request = self.client.get(&url);

        request = request.query(&[("first", limit.to_string().as_str())]);

        // Filter by category (game_id)
        if let Some(category) = filters.categories.first() {
            request = request.query(&[("game_id", category)]);
        }

        // Filter by language
        if let Some(language) = filters.languages.first() {
            request = request.query(&[("language", language)]);
        }

        for (key, value) in &headers {
            request = request.header(*key, value);
        }

        let response = request.send().await.map_err(|e| {
            ProviderError::HttpError(format!("Discovery request failed: {}", e))
        })?;

        let streams: StreamsResponse = response.json().await.map_err(|e| {
            ProviderError::ParseError(format!("Failed to parse discovery response: {}", e))
        })?;

        let discovered: Vec<DiscoveredStreamer> = streams
            .data
            .into_iter()
            .filter(|s| {
                // Filter by minimum viewers
                if let Some(min) = filters.min_viewers {
                    if s.viewer_count < min {
                        return false;
                    }
                }

                // Filter by maximum viewers
                if let Some(max) = filters.max_viewers {
                    if s.viewer_count > max {
                        return false;
                    }
                }

                // Filter by tags (client-side)
                if !filters.tags.is_empty() {
                    let has_any_tag = filters.tags.iter().any(|filter_tag| {
                        s.tags.iter().any(|stream_tag| {
                            stream_tag.to_lowercase() == filter_tag.to_lowercase()
                        })
                    });
                    if !has_any_tag {
                        return false;
                    }
                }

                true
            })
            .map(|s| DiscoveredStreamer {
                platform: "twitch".to_string(),
                user_id: s.user_id,
                display_name: s.user_name,
                is_live: true,
                viewer_count: Some(s.viewer_count),
                category: Some(s.game_name),
                tags: s.tags,
                language: Some(s.language),
            })
            .collect();

        Ok(discovered)
    }

    fn rate_limit_config(&self) -> RateLimitConfig {
        RateLimitConfig {
            requests_per_minute: 800,
            burst_size: 30,
        }
    }

    async fn health_check(&self) -> HealthStatus {
        match self.token_manager.get_token().await {
            Ok(_) => HealthStatus::Healthy,
            Err(e) => {
                error!("Twitch health check failed: {}", e);
                HealthStatus::Unhealthy
            }
        }
    }
}
