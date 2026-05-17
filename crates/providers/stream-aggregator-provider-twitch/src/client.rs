//! Twitch provider client implementation

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, error, warn};
use wreq::Client;

use stream_aggregator_core::{errors::ProviderError, models::*, traits::PlatformProvider};

use crate::auth::TokenManager;
use crate::models::{StreamsResponse, TwitchConfig, TwitchUser, UsersResponse};

const HELIX_API_BASE: &str = "https://api.twitch.tv/helix";
const USER_CACHE_TTL_HOURS: i64 = 24;

/// Cached user information
#[derive(Debug, Clone)]
struct CachedUserInfo {
    user: TwitchUser,
    cached_at: DateTime<Utc>,
}

impl CachedUserInfo {
    fn new(user: TwitchUser) -> Self {
        Self {
            user,
            cached_at: Utc::now(),
        }
    }

    fn is_stale(&self) -> bool {
        let age = Utc::now() - self.cached_at;
        age > Duration::hours(USER_CACHE_TTL_HOURS)
    }
}

/// Twitch platform provider
pub struct TwitchProvider {
    client: Client,
    config: TwitchConfig,
    token_manager: TokenManager,
    user_cache: Arc<RwLock<HashMap<String, CachedUserInfo>>>,
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
            user_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get headers for authenticated requests
    async fn auth_headers(&self) -> Result<HashMap<&'static str, String>, ProviderError> {
        let token =
            self.token_manager.get_token().await.map_err(|e| {
                ProviderError::AuthError(format!("Failed to get Twitch token: {}", e))
            })?;

        let mut headers = HashMap::new();
        headers.insert("Authorization", format!("Bearer {}", token));
        headers.insert("Client-Id", self.config.client_id.clone());
        Ok(headers)
    }

    /// Get cached user info if available and not stale
    fn get_cached_user(&self, user_id: &str) -> Option<TwitchUser> {
        let cache = self.user_cache.read().ok()?;
        let cached = cache.get(user_id)?;
        if cached.is_stale() {
            debug!("Cached user {} is stale", user_id);
            None
        } else {
            debug!("Using cached user {}", user_id);
            Some(cached.user.clone())
        }
    }

    /// Update cache with user info
    fn cache_user(&self, user: TwitchUser) {
        if let Ok(mut cache) = self.user_cache.write() {
            let user_id = user.id.clone();
            cache.insert(user_id.clone(), CachedUserInfo::new(user));
            debug!("Cached user {}", user_id);
        }
    }

    /// Batch update cache with multiple users
    fn cache_users(&self, users: Vec<TwitchUser>) {
        if let Ok(mut cache) = self.user_cache.write() {
            for user in users {
                let user_id = user.id.clone();
                cache.insert(user_id, CachedUserInfo::new(user));
            }
            debug!("Cached {} users", cache.len());
        }
    }

    /// Resolve a username (login) to a user ID
    pub async fn resolve_username_to_user_id(
        &self,
        username: &str,
    ) -> Result<String, ProviderError> {
        let headers = self.auth_headers().await?;

        let url = format!("{}/users", HELIX_API_BASE);
        let mut request = self.client.get(&url);

        request = request.query(&[("login", username)]);

        for (key, value) in headers {
            request = request.header(key, value);
        }

        let response = request
            .send()
            .await
            .map_err(|e| ProviderError::HttpError(format!("Failed to fetch Twitch user: {}", e)))?;

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
        debug!("Fetching stream info for user_id: {}", user_id);
        let headers = self.auth_headers().await?;

        // First, try to get user info from cache
        let user = if let Some(cached_user) = self.get_cached_user(user_id) {
            cached_user
        } else {
            // Cache miss - fetch from API
            let user_url = format!("{}/users?id={}", HELIX_API_BASE, user_id);
            debug!("GET {}", user_url);
            let mut user_request = self.client.get(&user_url);
            for (key, value) in &headers {
                user_request = user_request.header(*key, value);
            }

            let user_response = user_request.send().await.map_err(|e| {
                error!("Network error: {}", e);
                ProviderError::HttpError(format!("Failed to fetch Twitch user info: {}", e))
            })?;

            let status = user_response.status();
            debug!("Response status: {}", status);

            if !status.is_success() {
                let body = user_response.text().await.unwrap_or_default();
                error!("Twitch API error {} (user: {}): {}", status, user_id, body);
                return Err(ProviderError::HttpError(format!(
                    "Twitch API error {}: {}",
                    status, body
                )));
            }

            let users: UsersResponse = user_response.json().await.map_err(|e| {
                error!("Failed to parse Twitch user response: {}", e);
                ProviderError::ParseError(format!("Failed to parse Twitch user response: {}", e))
            })?;

            debug!("Found {} user(s)", users.data.len());
            if users.data.is_empty() {
                warn!("User not found in Twitch API: {}", user_id);
                return Err(ProviderError::StreamerNotFound(user_id.to_string()));
            }

            let fetched_user = users.data.into_iter().next().unwrap();
            self.cache_user(fetched_user.clone());
            fetched_user
        };

        // Then check if they're streaming
        let stream_url = format!("{}/streams?user_id={}", HELIX_API_BASE, user_id);
        debug!("GET {}", stream_url);
        let mut stream_request = self.client.get(&stream_url);
        for (key, value) in &headers {
            stream_request = stream_request.header(*key, value);
        }

        let stream_response = stream_request.send().await.map_err(|e| {
            error!("Network error: {}", e);
            ProviderError::HttpError(format!("Failed to fetch Twitch stream: {}", e))
        })?;

        let stream_status = stream_response.status();
        debug!("Response status: {}", stream_status);

        if !stream_status.is_success() {
            let body = stream_response.text().await.unwrap_or_default();
            error!(
                "Twitch stream API error {} (user: {}): {}",
                stream_status, user_id, body
            );
            return Err(ProviderError::HttpError(format!(
                "Twitch API error {}: {}",
                stream_status, body
            )));
        }

        let streams: StreamsResponse = stream_response.json().await.map_err(|e| {
            error!("Failed to parse Twitch streams response: {}", e);
            ProviderError::ParseError(format!("Failed to parse Twitch streams response: {}", e))
        })?;

        debug!("Found {} stream(s) for user", streams.data.len());

        let mut stream_info = StreamInfo::new("twitch", &user.id, &user.display_name);
        stream_info.login = Some(user.login.clone());
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
            let thumbnail = stream
                .thumbnail_url
                .replace("{width}", "1280")
                .replace("{height}", "720");
            stream_info.thumbnail_url = Some(thumbnail);
        } else {
            // User is offline
            stream_info.is_live = false;
        }

        stream_info.last_fetched_at = Utc::now();

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

    async fn fetch_streams_batch(
        &self,
        user_ids: &[String],
    ) -> Vec<Result<StreamInfo, ProviderError>> {
        if user_ids.is_empty() {
            return Vec::new();
        }

        debug!(count = user_ids.len(), "Batch fetching Twitch streams");

        // Twitch supports up to 100 user IDs per request
        let chunk_size = 100;
        let mut results = Vec::with_capacity(user_ids.len());

        for (chunk_idx, chunk) in user_ids.chunks(chunk_size).enumerate() {
            debug!(
                "Processing chunk {} with {} user(s)",
                chunk_idx,
                chunk.len()
            );
            let headers = match self.auth_headers().await {
                Ok(h) => {
                    debug!("Auth headers obtained for chunk {}", chunk_idx);
                    h
                }
                Err(e) => {
                    error!("Auth failed for chunk {}: {}", chunk_idx, e);
                    // If auth fails, return error for all in this chunk
                    for _user_id in chunk {
                        results.push(Err(e.clone()));
                    }
                    continue;
                }
            };

            // Step 1: Check cache and identify users that need fetching
            let mut user_map: HashMap<String, TwitchUser> = HashMap::new();
            let mut users_to_fetch = Vec::new();

            for user_id in chunk {
                if let Some(cached_user) = self.get_cached_user(user_id) {
                    user_map.insert(user_id.clone(), cached_user);
                } else {
                    users_to_fetch.push(user_id.as_str());
                }
            }

            debug!(
                "Cache hit: {}/{}, need to fetch: {}",
                user_map.len(),
                chunk.len(),
                users_to_fetch.len()
            );

            // Step 2: Batch fetch user information only for cache misses
            if !users_to_fetch.is_empty() {
                let url = format!("{}/users", HELIX_API_BASE);
                debug!("GET {} with {} user IDs", url, users_to_fetch.len());
                let mut request = self.client.get(&url);

                // Build all query params at once to avoid overwriting
                let query_params: Vec<(&str, &str)> =
                    users_to_fetch.iter().map(|id| ("id", *id)).collect();
                request = request.query(&query_params);

                for (key, value) in &headers {
                    request = request.header(*key, value);
                }

                let response = match request.send().await {
                    Ok(r) => {
                        debug!("Batch users request sent, response status: {}", r.status());
                        r
                    }
                    Err(e) => {
                        error!("Batch users request network error: {}", e);
                        let err =
                            ProviderError::HttpError(format!("Batch users request failed: {}", e));
                        for _user_id in chunk {
                            results.push(Err(err.clone()));
                        }
                        continue;
                    }
                };

                let status = response.status();
                if !status.is_success() {
                    let body = response.text().await.unwrap_or_default();
                    error!("Batch users request returned error {}: {}", status, body);
                    let err = ProviderError::HttpError(format!(
                        "Batch users request error {}: {}",
                        status, body
                    ));
                    for _user_id in chunk {
                        results.push(Err(err.clone()));
                    }
                    continue;
                }

                let users: UsersResponse = match response.json::<UsersResponse>().await {
                    Ok(u) => {
                        debug!(
                            "Batch users response parsed, found {} user(s)",
                            u.data.len()
                        );
                        u
                    }
                    Err(e) => {
                        error!("Failed to parse batch users response: {}", e);
                        let err = ProviderError::ParseError(format!(
                            "Failed to parse batch users response: {}",
                            e
                        ));
                        for _user_id in chunk {
                            results.push(Err(err.clone()));
                        }
                        continue;
                    }
                };

                // Add fetched users to map and cache them
                let fetched_users: Vec<TwitchUser> = users.data;
                for user in fetched_users {
                    user_map.insert(user.id.clone(), user.clone());
                }

                // Batch update cache
                self.cache_users(user_map.values().cloned().collect());
            }

            debug!("User map has {} entry/entries", user_map.len());

            // Step 2: Batch fetch stream information for all users in this chunk
            let streams_url = format!("{}/streams", HELIX_API_BASE);
            debug!("GET {} with {} user IDs", streams_url, chunk.len());
            let mut streams_request = self.client.get(&streams_url);

            // Build query params for streams
            let streams_query_params: Vec<(&str, &str)> =
                chunk.iter().map(|id| ("user_id", id.as_str())).collect();
            streams_request = streams_request.query(&streams_query_params);

            for (key, value) in &headers {
                streams_request = streams_request.header(*key, value);
            }

            let streams_response = match streams_request.send().await {
                Ok(r) => {
                    debug!(
                        "Batch streams request sent, response status: {}",
                        r.status()
                    );
                    r
                }
                Err(e) => {
                    error!("Batch streams request network error: {}", e);
                    // Return offline status for all users if stream fetch fails
                    for user_id in chunk {
                        if let Some(user) = user_map.get(user_id) {
                            let mut stream_info =
                                StreamInfo::new("twitch", &user.id, &user.display_name);
        stream_info.login = Some(user.login.clone());
                            stream_info.avatar_url = Some(user.profile_image_url.clone());
                            stream_info.is_live = false;
                            stream_info.last_fetched_at = Utc::now();
                            results.push(Ok(stream_info));
                        } else {
                            results.push(Err(ProviderError::StreamerNotFound(user_id.to_string())));
                        }
                    }
                    continue;
                }
            };

            let streams_status = streams_response.status();
            if !streams_status.is_success() {
                let body = streams_response.text().await.unwrap_or_default();
                error!(
                    "Batch streams request returned error {}: {}",
                    streams_status, body
                );
                // Return offline status for all users if stream fetch fails
                for user_id in chunk {
                    if let Some(user) = user_map.get(user_id) {
                        let mut stream_info =
                            StreamInfo::new("twitch", &user.id, &user.display_name);
        stream_info.login = Some(user.login.clone());
                        stream_info.avatar_url = Some(user.profile_image_url.clone());
                        stream_info.is_live = false;
                        stream_info.last_fetched_at = Utc::now();
                        results.push(Ok(stream_info));
                    } else {
                        results.push(Err(ProviderError::StreamerNotFound(user_id.to_string())));
                    }
                }
                continue;
            }

            let streams: StreamsResponse = match streams_response.json::<StreamsResponse>().await {
                Ok(s) => {
                    debug!(
                        "Batch streams response parsed, found {} stream(s)",
                        s.data.len()
                    );
                    s
                }
                Err(e) => {
                    error!("Failed to parse batch streams response: {}", e);
                    // Return offline status for all users if parse fails
                    for user_id in chunk {
                        if let Some(user) = user_map.get(user_id) {
                            let mut stream_info =
                                StreamInfo::new("twitch", &user.id, &user.display_name);
        stream_info.login = Some(user.login.clone());
                            stream_info.avatar_url = Some(user.profile_image_url.clone());
                            stream_info.is_live = false;
                            stream_info.last_fetched_at = Utc::now();
                            results.push(Ok(stream_info));
                        } else {
                            results.push(Err(ProviderError::StreamerNotFound(user_id.to_string())));
                        }
                    }
                    continue;
                }
            };

            // Create a map of user_id -> stream for quick lookup
            let stream_map: HashMap<_, _> = streams
                .data
                .into_iter()
                .map(|s| (s.user_id.clone(), s))
                .collect();

            debug!("Stream map has {} entry/entries", stream_map.len());

            // Step 3: Combine user and stream data for each requested user_id
            for user_id in chunk {
                if let Some(user) = user_map.get(user_id) {
                    let mut stream_info = StreamInfo::new("twitch", &user.id, &user.display_name);
        stream_info.login = Some(user.login.clone());
                    stream_info.avatar_url = Some(user.profile_image_url.clone());

                    if let Some(stream) = stream_map.get(user_id) {
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
                        let thumbnail = stream
                            .thumbnail_url
                            .replace("{width}", "1280")
                            .replace("{height}", "720");
                        stream_info.thumbnail_url = Some(thumbnail);
                    } else {
                        // User is offline
                        stream_info.is_live = false;
                    }

                    stream_info.last_fetched_at = Utc::now();
                    debug!(
                        "Successfully processed stream for {} (live: {})",
                        user_id, stream_info.is_live
                    );
                    results.push(Ok(stream_info));
                } else {
                    warn!("User {} not found in Twitch API response", user_id);
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

        let response = request
            .send()
            .await
            .map_err(|e| ProviderError::HttpError(format!("Discovery request failed: {}", e)))?;

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
