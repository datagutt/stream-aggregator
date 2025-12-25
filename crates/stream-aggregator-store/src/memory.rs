//! In-memory storage implementation using DashMap

use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use tracing::{debug, trace};

use stream_aggregator_core::{errors::StoreError, models::*, traits::StreamStore};

/// In-memory storage implementation
///
/// Uses DashMap for concurrent access without locking.
/// Data is ephemeral and will be lost on restart.
#[derive(Clone)]
pub struct MemoryStore {
    streams: Arc<DashMap<String, StreamInfo>>,
    tracked_streamers: Arc<DashMap<String, TrackedStreamer>>,
    discovery_rules: Arc<DashMap<String, DiscoveryRule>>,
}

impl MemoryStore {
    /// Create a new in-memory store
    pub fn new() -> Self {
        debug!("Creating new MemoryStore");
        Self {
            streams: Arc::new(DashMap::new()),
            tracked_streamers: Arc::new(DashMap::new()),
            discovery_rules: Arc::new(DashMap::new()),
        }
    }

    /// Helper function to create a key for tracked streamers
    fn streamer_key(platform: &str, user_id: &str) -> String {
        format!("{}:{}", platform, user_id)
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StreamStore for MemoryStore {
    async fn upsert_stream(&self, stream: &StreamInfo) -> Result<(), StoreError> {
        trace!(stream_id = %stream.id, platform = %stream.platform, "Upserting stream");
        self.streams.insert(stream.id.0.clone(), stream.clone());
        Ok(())
    }

    async fn get_stream(&self, id: &StreamId) -> Result<Option<StreamInfo>, StoreError> {
        trace!(stream_id = %id, "Getting stream");
        Ok(self.streams.get(&id.0).map(|entry| entry.clone()))
    }

    async fn get_streams(&self, query: &StreamQuery) -> Result<StreamPage, StoreError> {
        trace!(?query, "Querying streams");

        // Filter streams
        let mut filtered: Vec<StreamInfo> = self
            .streams
            .iter()
            .map(|entry| entry.value().clone())
            .filter(|stream| {
                // Filter by platform
                if let Some(ref platform) = query.platform {
                    if &stream.platform != platform {
                        return false;
                    }
                }

                // Filter by live status
                if let Some(is_live) = query.is_live {
                    if stream.is_live != is_live {
                        return false;
                    }
                }

                // Filter by group (from metadata)
                if let Some(ref group) = query.group {
                    if let Some(metadata_group) =
                        stream.metadata.get("group").and_then(|v| v.as_str())
                    {
                        if metadata_group != group {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                // Filter by labels (from metadata)
                for (key, value) in &query.labels {
                    if let Some(metadata_labels) =
                        stream.metadata.get("labels").and_then(|v| v.as_object())
                    {
                        if metadata_labels.get(key).and_then(|v| v.as_str()) != Some(value) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                // Search in display name and title
                if let Some(ref search) = query.search {
                    let search_lower = search.to_lowercase();
                    let display_name_match =
                        stream.display_name.to_lowercase().contains(&search_lower);
                    let title_match = stream
                        .title
                        .as_ref()
                        .map(|t| t.to_lowercase().contains(&search_lower))
                        .unwrap_or(false);

                    if !display_name_match && !title_match {
                        return false;
                    }
                }

                // Filter by language
                if let Some(ref language) = query.language {
                    if stream.language.as_ref() != Some(language) {
                        return false;
                    }
                }

                // Filter by category
                if let Some(ref category) = query.category {
                    if stream.category.as_ref() != Some(category) {
                        return false;
                    }
                }

                // Filter by tag
                if let Some(ref tag) = query.tag {
                    if !stream.tags.contains(tag) {
                        return false;
                    }
                }

                // Filter by minimum viewers
                if let Some(min_viewers) = query.min_viewers {
                    if stream.viewer_count.unwrap_or(0) < min_viewers {
                        return false;
                    }
                }

                // Filter by maximum viewers
                if let Some(max_viewers) = query.max_viewers {
                    if stream.viewer_count.unwrap_or(0) > max_viewers {
                        return false;
                    }
                }

                true
            })
            .collect();

        let total = filtered.len();

        // Apply sorting
        let sort_field = query.sort.as_deref().unwrap_or("viewers");
        let is_ascending = query.order.as_deref().unwrap_or("desc") == "asc";

        filtered.sort_by(|a, b| {
            let ordering = match sort_field {
                "name" => a.display_name.cmp(&b.display_name),
                "platform" => a.platform.cmp(&b.platform),
                "updated" => a.last_updated.cmp(&b.last_updated),
                "viewers" => b
                    .viewer_count
                    .unwrap_or(0)
                    .cmp(&a.viewer_count.unwrap_or(0)),
                _ => a.display_name.cmp(&b.display_name), // Default to name
            };

            if is_ascending {
                ordering
            } else {
                ordering.reverse()
            }
        });

        // Pagination
        let page = query.page.unwrap_or(0);
        let page_size = query.page_size.unwrap_or(50).min(100); // Max 100 items per page
        let total_pages = total.div_ceil(page_size);

        let start = page * page_size;
        let end = (start + page_size).min(total);

        let items = if start < total {
            filtered[start..end].to_vec()
        } else {
            Vec::new()
        };

        Ok(StreamPage {
            items,
            total,
            page,
            page_size,
            total_pages,
        })
    }

    async fn delete_stream(&self, id: &StreamId) -> Result<(), StoreError> {
        trace!(stream_id = %id, "Deleting stream");
        self.streams.remove(&id.0);
        Ok(())
    }

    async fn add_tracked_streamer(&self, streamer: &TrackedStreamer) -> Result<(), StoreError> {
        let key = Self::streamer_key(&streamer.platform, &streamer.user_id);
        trace!(key = %key, "Adding tracked streamer");

        if self.tracked_streamers.contains_key(&key) {
            return Err(StoreError::AlreadyExists(format!(
                "Streamer {} on {} is already tracked",
                streamer.user_id, streamer.platform
            )));
        }

        self.tracked_streamers.insert(key, streamer.clone());
        Ok(())
    }

    async fn get_tracked_streamer(
        &self,
        platform: &str,
        user_id: &str,
    ) -> Result<Option<TrackedStreamer>, StoreError> {
        let key = Self::streamer_key(platform, user_id);
        trace!(key = %key, "Getting tracked streamer");
        Ok(self.tracked_streamers.get(&key).map(|entry| entry.clone()))
    }

    async fn get_tracked_streamers(
        &self,
        query: &TrackedStreamerQuery,
    ) -> Result<Vec<TrackedStreamer>, StoreError> {
        trace!(?query, "Querying tracked streamers");

        let streamers: Vec<TrackedStreamer> = self
            .tracked_streamers
            .iter()
            .map(|entry| entry.value().clone())
            .filter(|streamer| {
                // Filter by platform
                if let Some(ref platform) = query.platform {
                    if &streamer.platform != platform {
                        return false;
                    }
                }

                // Filter by group
                if let Some(ref group) = query.group {
                    if streamer.group.as_ref() != Some(group) {
                        return false;
                    }
                }

                // Filter by source
                if let Some(ref source) = query.source {
                    if &streamer.source != source {
                        return false;
                    }
                }

                // Filter by labels
                for (key, value) in &query.labels {
                    if streamer.labels.get(key) != Some(value) {
                        return false;
                    }
                }

                true
            })
            .collect();

        Ok(streamers)
    }

    async fn remove_tracked_streamer(
        &self,
        platform: &str,
        user_id: &str,
    ) -> Result<(), StoreError> {
        let key = Self::streamer_key(platform, user_id);
        trace!(key = %key, "Removing tracked streamer");
        self.tracked_streamers.remove(&key);
        Ok(())
    }

    async fn update_tracked_streamer(&self, streamer: &TrackedStreamer) -> Result<(), StoreError> {
        let key = Self::streamer_key(&streamer.platform, &streamer.user_id);
        trace!(key = %key, "Updating tracked streamer");

        if !self.tracked_streamers.contains_key(&key) {
            return Err(StoreError::NotFound(format!(
                "Streamer {} on {} not found",
                streamer.user_id, streamer.platform
            )));
        }

        self.tracked_streamers.insert(key, streamer.clone());
        Ok(())
    }

    async fn add_discovery_rule(&self, rule: &DiscoveryRule) -> Result<(), StoreError> {
        trace!(rule_id = %rule.id, "Adding discovery rule");

        if self.discovery_rules.contains_key(&rule.id) {
            return Err(StoreError::AlreadyExists(format!(
                "Discovery rule {} already exists",
                rule.id
            )));
        }

        self.discovery_rules.insert(rule.id.clone(), rule.clone());
        Ok(())
    }

    async fn get_discovery_rule(&self, id: &str) -> Result<Option<DiscoveryRule>, StoreError> {
        trace!(rule_id = %id, "Getting discovery rule");
        Ok(self.discovery_rules.get(id).map(|entry| entry.clone()))
    }

    async fn get_discovery_rules(
        &self,
        platform: Option<&str>,
    ) -> Result<Vec<DiscoveryRule>, StoreError> {
        trace!(?platform, "Querying discovery rules");

        let rules: Vec<DiscoveryRule> = self
            .discovery_rules
            .iter()
            .map(|entry| entry.value().clone())
            .filter(|rule| {
                if let Some(platform) = platform {
                    rule.platform == platform
                } else {
                    true
                }
            })
            .collect();

        Ok(rules)
    }

    async fn update_discovery_rule(&self, rule: &DiscoveryRule) -> Result<(), StoreError> {
        trace!(rule_id = %rule.id, "Updating discovery rule");

        if !self.discovery_rules.contains_key(&rule.id) {
            return Err(StoreError::NotFound(format!(
                "Discovery rule {} not found",
                rule.id
            )));
        }

        self.discovery_rules.insert(rule.id.clone(), rule.clone());
        Ok(())
    }

    async fn remove_discovery_rule(&self, id: &str) -> Result<(), StoreError> {
        trace!(rule_id = %id, "Removing discovery rule");
        self.discovery_rules.remove(id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stream_operations() {
        let store = MemoryStore::new();

        // Create a stream
        let stream = StreamInfo::new("twitch", "user123", "TestUser");
        let id = stream.id.clone();

        // Upsert
        store.upsert_stream(&stream).await.unwrap();

        // Get
        let retrieved = store.get_stream(&id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().display_name, "TestUser");

        // Delete
        store.delete_stream(&id).await.unwrap();
        let deleted = store.get_stream(&id).await.unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_tracked_streamer_operations() {
        let store = MemoryStore::new();

        // Add streamer
        let streamer = TrackedStreamer::new_manual("twitch", "user123");
        store.add_tracked_streamer(&streamer).await.unwrap();

        // Get streamer
        let retrieved = store
            .get_tracked_streamer("twitch", "user123")
            .await
            .unwrap();
        assert!(retrieved.is_some());

        // Try to add again (should fail)
        let result = store.add_tracked_streamer(&streamer).await;
        assert!(result.is_err());

        // Remove streamer
        store
            .remove_tracked_streamer("twitch", "user123")
            .await
            .unwrap();
        let deleted = store
            .get_tracked_streamer("twitch", "user123")
            .await
            .unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_discovery_rule_operations() {
        let store = MemoryStore::new();

        // Add rule
        let rule = DiscoveryRule::new("rule1", "Test Rule", "twitch");
        store.add_discovery_rule(&rule).await.unwrap();

        // Get rule
        let retrieved = store.get_discovery_rule("rule1").await.unwrap();
        assert!(retrieved.is_some());

        // Get all rules
        let rules = store.get_discovery_rules(None).await.unwrap();
        assert_eq!(rules.len(), 1);

        // Remove rule
        store.remove_discovery_rule("rule1").await.unwrap();
        let deleted = store.get_discovery_rule("rule1").await.unwrap();
        assert!(deleted.is_none());
    }
}
