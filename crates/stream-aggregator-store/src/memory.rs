//! In-memory storage implementation using DashMap

use async_trait::async_trait;
use chrono::Utc;
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
    /// Communities by slug. `domains` field on each Community is the source of truth.
    communities: Arc<DashMap<String, Community>>,
    /// Reverse index: host -> slug, kept in sync with `communities`.
    community_domain_index: Arc<DashMap<String, String>>,
}

impl MemoryStore {
    /// Create a new in-memory store
    pub fn new() -> Self {
        debug!("Creating new MemoryStore");
        Self {
            streams: Arc::new(DashMap::new()),
            tracked_streamers: Arc::new(DashMap::new()),
            discovery_rules: Arc::new(DashMap::new()),
            communities: Arc::new(DashMap::new()),
            community_domain_index: Arc::new(DashMap::new()),
        }
    }

    /// Helper function to create a key for tracked streamers
    fn streamer_key(platform: &str, user_id: &str) -> String {
        format!("{}:{}", platform, user_id)
    }

    /// Merge `last_live_at` from the existing row into the incoming stream.
    /// Storage layer is the single chokepoint that sees prior state.
    fn merge_last_live_at(&self, incoming: &mut StreamInfo) {
        let existing = self.streams.get(&incoming.id.0);
        let prior_last_live = existing.as_ref().and_then(|e| e.last_live_at);
        let prior_was_live = existing.as_ref().map(|e| e.is_live).unwrap_or(false);

        incoming.last_live_at = StreamInfo::merge_last_live_at(
            incoming.is_live,
            incoming.started_at,
            prior_last_live,
            prior_was_live,
            Utc::now(),
        );
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
        let mut merged = stream.clone();
        self.merge_last_live_at(&mut merged);
        self.streams.insert(merged.id.0.clone(), merged);
        Ok(())
    }

    async fn batch_upsert_streams(&self, streams: &[StreamInfo]) -> Result<(), StoreError> {
        debug!("Batch upserting {} streams", streams.len());
        for stream in streams {
            let mut merged = stream.clone();
            self.merge_last_live_at(&mut merged);
            self.streams.insert(merged.id.0.clone(), merged);
        }
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
                // Filter by platforms (any-of)
                if !query.platforms.is_empty() && !query.platforms.contains(&stream.platform) {
                    return false;
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

                // Filter by languages (any-of)
                if !query.languages.is_empty() {
                    match stream.language.as_ref() {
                        Some(lang) if query.languages.contains(lang) => {}
                        _ => return false,
                    }
                }

                // Filter by categories (any-of)
                if !query.categories.is_empty() {
                    match stream.category.as_ref() {
                        Some(cat) if query.categories.contains(cat) => {}
                        _ => return false,
                    }
                }

                // Filter by tags (stream matches when it contains AT LEAST ONE of the requested tags)
                if !query.tags.is_empty()
                    && !query.tags.iter().any(|t| stream.tags.contains(t))
                {
                    return false;
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
                "updated" | "fetched" => a.last_fetched_at.cmp(&b.last_fetched_at),
                "live" => a.last_live_at.cmp(&b.last_live_at),
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

    // ===== Community Operations =====

    async fn list_communities(&self) -> Result<Vec<Community>, StoreError> {
        Ok(self
            .communities
            .iter()
            .map(|entry| entry.value().clone())
            .collect())
    }

    async fn get_community(&self, slug: &str) -> Result<Option<Community>, StoreError> {
        Ok(self.communities.get(slug).map(|entry| entry.value().clone()))
    }

    async fn get_community_by_domain(
        &self,
        host: &str,
    ) -> Result<Option<Community>, StoreError> {
        let Some(slug) = self.community_domain_index.get(host).map(|s| s.clone()) else {
            return Ok(None);
        };
        Ok(self.communities.get(&slug).map(|entry| entry.value().clone()))
    }

    async fn upsert_community(&self, community: &Community) -> Result<Community, StoreError> {
        let now = Utc::now();
        let mut stored = community.clone();
        stored.updated_at = now;
        if !self.communities.contains_key(&stored.slug) {
            stored.created_at = now;
        } else if let Some(existing) = self.communities.get(&stored.slug) {
            stored.created_at = existing.created_at;
        }

        // Atomically replace this community's entries in the domain index.
        // Reject if any incoming domain is already claimed by a different community.
        for host in &stored.domains {
            if let Some(existing_slug) = self.community_domain_index.get(host) {
                if existing_slug.value() != &stored.slug {
                    return Err(StoreError::QueryError(format!(
                        "domain '{host}' is already claimed by community '{}'",
                        existing_slug.value()
                    )));
                }
            }
        }

        // Drop any previously-mapped hosts no longer in the incoming set.
        let new_hosts: std::collections::HashSet<&String> = stored.domains.iter().collect();
        self.community_domain_index
            .retain(|_, slug| slug != &stored.slug);
        for host in &stored.domains {
            let _ = new_hosts;
            self.community_domain_index
                .insert(host.clone(), stored.slug.clone());
        }

        self.communities.insert(stored.slug.clone(), stored.clone());
        Ok(stored)
    }

    async fn delete_community(&self, slug: &str) -> Result<bool, StoreError> {
        let existed = self.communities.remove(slug).is_some();
        // Cascade through the domain index.
        self.community_domain_index
            .retain(|_, mapped_slug| mapped_slug != slug);
        Ok(existed)
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
    async fn test_multi_value_filters() {
        let store = MemoryStore::new();

        let mut a = StreamInfo::new("twitch", "a", "Alice");
        a.language = Some("no".into());
        a.category = Some("Just Chatting".into());
        a.tags = vec!["nordic".into()];
        a.is_live = true;

        let mut b = StreamInfo::new("youtube", "b", "Bob");
        b.language = Some("sv".into());
        b.category = Some("Music".into());
        b.tags = vec!["acoustic".into(), "nordic".into()];
        b.is_live = true;

        let mut c = StreamInfo::new("kick", "c", "Cara");
        c.language = Some("en".into());
        c.tags = vec!["fps".into()];
        c.is_live = true;

        for s in [&a, &b, &c] {
            store.upsert_stream(s).await.unwrap();
        }

        // Scandinavian directory: two languages
        let page = store
            .get_streams(&StreamQuery {
                languages: vec!["no".into(), "sv".into()],
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(page.total, 2);

        // Two platforms
        let page = store
            .get_streams(&StreamQuery {
                platforms: vec!["twitch".into(), "kick".into()],
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(page.total, 2);

        // Any-of tags
        let page = store
            .get_streams(&StreamQuery {
                tags: vec!["nordic".into(), "fps".into()],
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(page.total, 3);

        // Empty vectors mean "no filter"
        let page = store
            .get_streams(&StreamQuery::default())
            .await
            .unwrap();
        assert_eq!(page.total, 3);
    }

    #[tokio::test]
    async fn test_community_crud() {
        let store = MemoryStore::new();

        let mut c = Community::new("lsn", "LiveStreamNorge", "0.58 0.20 250");
        c.domains = vec!["lsn.local".into(), "livestreamnorge.test".into()];
        c.filter.languages = vec!["no".into()];

        // Create
        let created = store.upsert_community(&c).await.unwrap();
        assert_eq!(created.slug, "lsn");
        assert_eq!(created.domains.len(), 2);

        // List
        let all = store.list_communities().await.unwrap();
        assert_eq!(all.len(), 1);

        // Get by slug
        let got = store.get_community("lsn").await.unwrap().unwrap();
        assert_eq!(got.name, "LiveStreamNorge");

        // Get by domain
        let by_host = store
            .get_community_by_domain("lsn.local")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(by_host.slug, "lsn");
        assert!(store
            .get_community_by_domain("missing.test")
            .await
            .unwrap()
            .is_none());

        // Update (replace domains)
        let mut updated = got.clone();
        updated.tagline = Some("Norske strømmere live nå".into());
        updated.domains = vec!["lsn.local".into()];
        let saved = store.upsert_community(&updated).await.unwrap();
        assert_eq!(saved.tagline.as_deref(), Some("Norske strømmere live nå"));
        assert_eq!(saved.domains, vec!["lsn.local".to_string()]);
        assert!(store
            .get_community_by_domain("livestreamnorge.test")
            .await
            .unwrap()
            .is_none());

        // Domain conflict: a different community can't claim lsn.local
        let mut other = Community::new("sv", "SwedishStreamers", "0.78 0.16 95");
        other.domains = vec!["lsn.local".into()];
        let err = store.upsert_community(&other).await.unwrap_err();
        assert!(format!("{err}").contains("already claimed"));

        // Delete cascades through the domain index
        assert!(store.delete_community("lsn").await.unwrap());
        assert!(store
            .get_community_by_domain("lsn.local")
            .await
            .unwrap()
            .is_none());
        assert!(!store.delete_community("lsn").await.unwrap());
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
