//! Core traits for StreamAggregator

use async_trait::async_trait;

use crate::errors::{ProviderError, StoreError};
use crate::models::*;

/// Trait for platform providers
///
/// Each streaming platform implements this trait to provide stream information
/// and discovery capabilities.
#[async_trait]
pub trait PlatformProvider: Send + Sync + 'static {
    /// Unique identifier for this platform (e.g., "twitch", "youtube")
    fn platform_id(&self) -> &'static str;

    /// Human-readable display name (e.g., "Twitch", "YouTube")
    fn display_name(&self) -> &'static str;

    /// Base URL for the platform (e.g., "https://twitch.tv")
    fn base_url(&self) -> &'static str;

    /// Fetch stream information for a specific streamer
    ///
    /// # Arguments
    /// * `streamer_id` - Platform-specific user ID or username
    ///
    /// # Returns
    /// * `Ok(StreamInfo)` - Stream information (may have is_live=false if offline)
    /// * `Err(ProviderError)` - If the request fails or streamer doesn't exist
    async fn fetch_stream(&self, streamer_id: &str) -> Result<StreamInfo, ProviderError>;

    /// Batch fetch multiple streamers (optimization)
    ///
    /// Default implementation calls `fetch_stream` for each ID sequentially.
    /// Providers that support batch requests should override this.
    ///
    /// # Arguments
    /// * `streamer_ids` - List of platform-specific user IDs or usernames
    ///
    /// # Returns
    /// * Vector of results (one per input ID, in the same order)
    async fn fetch_streams_batch(&self, streamer_ids: &[String]) -> Vec<Result<StreamInfo, ProviderError>> {
        let mut results = Vec::with_capacity(streamer_ids.len());
        for id in streamer_ids {
            results.push(self.fetch_stream(id).await);
        }
        results
    }

    /// Check if this provider supports automatic discovery
    fn supports_discovery(&self) -> bool {
        false
    }

    /// Discover streamers matching the given filters
    ///
    /// Only providers that return `true` from `supports_discovery()` need to implement this.
    ///
    /// # Arguments
    /// * `filters` - Discovery filters (tags, categories, languages, etc.)
    ///
    /// # Returns
    /// * `Ok(Vec<DiscoveredStreamer>)` - List of discovered streamers
    /// * `Err(ProviderError::DiscoveryNotSupported)` - If discovery is not supported
    async fn discover_streamers(
        &self,
        _filters: &DiscoveryFilters,
    ) -> Result<Vec<DiscoveredStreamer>, ProviderError> {
        Err(ProviderError::DiscoveryNotSupported)
    }

    /// Get rate limit configuration for this provider
    fn rate_limit_config(&self) -> RateLimitConfig {
        RateLimitConfig::default()
    }

    /// Perform a health check on this provider
    ///
    /// Default implementation returns Healthy. Providers can override to check
    /// API availability, authentication status, etc.
    async fn health_check(&self) -> HealthStatus {
        HealthStatus::Healthy
    }
}

/// Trait for storage backends
///
/// Provides an abstraction over different storage implementations
/// (in-memory, SQLite, PostgreSQL, etc.)
#[async_trait]
pub trait StreamStore: Send + Sync + 'static {
    // ===== Stream Operations =====

    /// Insert or update stream information
    ///
    /// # Arguments
    /// * `stream` - Stream information to store
    async fn upsert_stream(&self, stream: &StreamInfo) -> Result<(), StoreError>;

    /// Get stream information by ID
    ///
    /// # Arguments
    /// * `id` - Unique stream ID
    ///
    /// # Returns
    /// * `Ok(Some(StreamInfo))` - If stream exists
    /// * `Ok(None)` - If stream doesn't exist
    async fn get_stream(&self, id: &StreamId) -> Result<Option<StreamInfo>, StoreError>;

    /// Get stream information by platform and user_id
    ///
    /// # Arguments
    /// * `platform` - Platform identifier
    /// * `user_id` - Platform-specific user ID
    ///
    /// # Returns
    /// * `Ok(Some(StreamInfo))` - If stream exists
    /// * `Ok(None)` - If stream doesn't exist
    async fn get_stream_by_platform_user(
        &self,
        platform: &str,
        user_id: &str,
    ) -> Result<Option<StreamInfo>, StoreError> {
        let id = StreamId::new(platform, user_id);
        self.get_stream(&id).await
    }

    /// Query streams with filters and pagination
    ///
    /// # Arguments
    /// * `query` - Query parameters (filters, pagination)
    ///
    /// # Returns
    /// * Paginated list of streams
    async fn get_streams(&self, query: &StreamQuery) -> Result<StreamPage, StoreError>;

    /// Delete stream information
    ///
    /// # Arguments
    /// * `id` - Unique stream ID
    async fn delete_stream(&self, id: &StreamId) -> Result<(), StoreError>;

    // ===== Tracked Streamer Operations =====

    /// Add a streamer to be tracked
    ///
    /// # Arguments
    /// * `streamer` - Tracked streamer information
    ///
    /// # Returns
    /// * `Ok(())` - If successful
    /// * `Err(StoreError::AlreadyExists)` - If streamer is already tracked
    async fn add_tracked_streamer(&self, streamer: &TrackedStreamer) -> Result<(), StoreError>;

    /// Get a specific tracked streamer
    ///
    /// # Arguments
    /// * `platform` - Platform identifier
    /// * `user_id` - Platform-specific user ID
    ///
    /// # Returns
    /// * `Ok(Some(TrackedStreamer))` - If streamer is tracked
    /// * `Ok(None)` - If streamer is not tracked
    async fn get_tracked_streamer(
        &self,
        platform: &str,
        user_id: &str,
    ) -> Result<Option<TrackedStreamer>, StoreError>;

    /// Query tracked streamers with filters
    ///
    /// # Arguments
    /// * `query` - Query parameters (platform, group, source, labels)
    ///
    /// # Returns
    /// * List of tracked streamers matching the query
    async fn get_tracked_streamers(
        &self,
        query: &TrackedStreamerQuery,
    ) -> Result<Vec<TrackedStreamer>, StoreError>;

    /// Remove a tracked streamer
    ///
    /// # Arguments
    /// * `platform` - Platform identifier
    /// * `user_id` - Platform-specific user ID
    async fn remove_tracked_streamer(&self, platform: &str, user_id: &str) -> Result<(), StoreError>;

    /// Update a tracked streamer
    ///
    /// # Arguments
    /// * `streamer` - Updated tracked streamer information
    async fn update_tracked_streamer(&self, streamer: &TrackedStreamer) -> Result<(), StoreError>;

    // ===== Discovery Rule Operations =====

    /// Add a discovery rule
    ///
    /// # Arguments
    /// * `rule` - Discovery rule to add
    ///
    /// # Returns
    /// * `Ok(())` - If successful
    /// * `Err(StoreError::AlreadyExists)` - If a rule with this ID already exists
    async fn add_discovery_rule(&self, rule: &DiscoveryRule) -> Result<(), StoreError>;

    /// Get a specific discovery rule by ID
    ///
    /// # Arguments
    /// * `id` - Rule ID
    ///
    /// # Returns
    /// * `Ok(Some(DiscoveryRule))` - If rule exists
    /// * `Ok(None)` - If rule doesn't exist
    async fn get_discovery_rule(&self, id: &str) -> Result<Option<DiscoveryRule>, StoreError>;

    /// Get all discovery rules (optionally filtered by platform)
    ///
    /// # Arguments
    /// * `platform` - Optional platform filter
    ///
    /// # Returns
    /// * List of discovery rules
    async fn get_discovery_rules(&self, platform: Option<&str>) -> Result<Vec<DiscoveryRule>, StoreError>;

    /// Update a discovery rule
    ///
    /// # Arguments
    /// * `rule` - Updated rule
    async fn update_discovery_rule(&self, rule: &DiscoveryRule) -> Result<(), StoreError>;

    /// Remove a discovery rule
    ///
    /// # Arguments
    /// * `id` - Rule ID
    async fn remove_discovery_rule(&self, id: &str) -> Result<(), StoreError>;
}
