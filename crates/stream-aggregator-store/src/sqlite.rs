//! SQLite storage implementation

use async_trait::async_trait;
use sqlx::{sqlite::SqliteConnectOptions, Row, SqlitePool};
use std::sync::Arc;
use tracing::{debug, trace};

use stream_aggregator_core::{
    errors::StoreError,
    models::*,
    traits::StreamStore,
};

/// SQLite storage implementation
///
/// Provides persistent storage using SQLite database.
/// All data is persisted to a single file.
#[derive(Clone)]
pub struct SqliteStore {
    pool: Arc<SqlitePool>,
}

impl SqliteStore {
    /// Create a new SQLite store with connection string
    pub async fn new(database_url: &str) -> Result<Self, StoreError> {
        debug!("Creating new SqliteStore with database: {}", database_url);
        
        // Configure connection options
        let options = SqliteConnectOptions::new()
            .filename(database_url)
            .create_if_missing(true);
            
        // Create connection pool
        let pool = SqlitePool::connect_with(options)
            .await
            .map_err(|e| StoreError::ConnectionError(e.to_string()))?;
            
        let store = Self {
            pool: Arc::new(pool),
        };
        
        // Run migrations
        store.migrate().await?;
        
        Ok(store)
    }
    
    /// Create a new SQLite store with in-memory database
    pub async fn memory() -> Result<Self, StoreError> {
        Self::new("file::memory:?cache=shared").await
    }
    
    /// Run database migrations
    async fn migrate(&self) -> Result<(), StoreError> {
        debug!("Running SQLite migrations");
        
        // Create streams table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS streams (
                id TEXT PRIMARY KEY,
                platform TEXT NOT NULL,
                user_id TEXT NOT NULL,
                display_name TEXT NOT NULL,
                avatar_url TEXT,
                is_live BOOLEAN NOT NULL DEFAULT FALSE,
                title TEXT,
                viewer_count INTEGER,
                thumbnail_url TEXT,
                category TEXT,
                tags TEXT NOT NULL DEFAULT '[]',
                language TEXT,
                started_at TEXT,
                last_updated TEXT NOT NULL,
                metadata TEXT NOT NULL DEFAULT '{}',
                UNIQUE(platform, user_id)
            )
            "#,
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| StoreError::QueryError(e.to_string()))?;
        
        // Create tracked_streamers table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tracked_streamers (
                platform TEXT NOT NULL,
                user_id TEXT NOT NULL,
                custom_name TEXT,
                group_name TEXT,
                priority INTEGER,
                labels TEXT NOT NULL DEFAULT '{}',
                source TEXT NOT NULL DEFAULT 'manual',
                discovery_rule_id TEXT,
                created_at TEXT NOT NULL,
                PRIMARY KEY (platform, user_id)
            )
            "#,
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| StoreError::QueryError(e.to_string()))?;
        
        // Create discovery_rules table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS discovery_rules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                platform TEXT NOT NULL,
                enabled BOOLEAN NOT NULL DEFAULT TRUE,
                filters TEXT NOT NULL DEFAULT '{}',
                interval_secs INTEGER NOT NULL,
                apply_labels TEXT NOT NULL DEFAULT '{}',
                apply_group TEXT,
                created_at TEXT NOT NULL,
                last_run_at TEXT
            )
            "#,
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| StoreError::QueryError(e.to_string()))?;
        
        // Create indexes for better query performance
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_streams_platform ON streams(platform)")
            .execute(&*self.pool)
            .await
            .map_err(|e| StoreError::QueryError(e.to_string()))?;
            
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_streams_is_live ON streams(is_live)")
            .execute(&*self.pool)
            .await
            .map_err(|e| StoreError::QueryError(e.to_string()))?;
            
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_streams_viewer_count ON streams(viewer_count)")
            .execute(&*self.pool)
            .await
            .map_err(|e| StoreError::QueryError(e.to_string()))?;
            
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tracked_streamers_platform ON tracked_streamers(platform)")
            .execute(&*self.pool)
            .await
            .map_err(|e| StoreError::QueryError(e.to_string()))?;
            
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_discovery_rules_platform ON discovery_rules(platform)")
            .execute(&*self.pool)
            .await
            .map_err(|e| StoreError::QueryError(e.to_string()))?;
        
        debug!("✅ SQLite migrations completed");
        Ok(())
    }
}

#[async_trait]
impl StreamStore for SqliteStore {
    async fn upsert_stream(&self, stream: &StreamInfo) -> Result<(), StoreError> {
        trace!(stream_id = %stream.id, platform = %stream.platform, "Upserting stream");
        
        let tags_json = serde_json::to_string(&stream.tags)
            .map_err(|e| StoreError::SerializationError(e.to_string()))?;
        let metadata_json = serde_json::to_string(&stream.metadata)
            .map_err(|e| StoreError::SerializationError(e.to_string()))?;
            
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO streams (
                id, platform, user_id, display_name, avatar_url, title, category, language,
                is_live, viewer_count, thumbnail_url, tags, metadata, started_at, last_updated
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&stream.id.0)
        .bind(&stream.platform)
        .bind(&stream.user_id)
        .bind(&stream.display_name)
        .bind(&stream.avatar_url)
        .bind(&stream.title)
        .bind(&stream.category)
        .bind(&stream.language)
        .bind(stream.is_live)
        .bind(stream.viewer_count.map(|v| v as i64))
        .bind(&stream.thumbnail_url)
        .bind(tags_json)
        .bind(metadata_json)
        .bind(stream.started_at.map(|dt| dt.to_rfc3339()))
        .bind(stream.last_updated.to_rfc3339())
        .execute(&*self.pool)
        .await
        .map_err(|e| StoreError::QueryError(e.to_string()))?;
        
        Ok(())
    }

    async fn get_stream(&self, id: &StreamId) -> Result<Option<StreamInfo>, StoreError> {
        trace!(stream_id = %id, "Getting stream");
        
        let row = sqlx::query("SELECT * FROM streams WHERE id = ?")
            .bind(&id.0)
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| StoreError::QueryError(e.to_string()))?;
            
        match row {
            Some(row) => {
                let tags: Vec<String> = serde_json::from_str(
                    row.get::<&str, _>("tags")
                ).unwrap_or_default();
                let metadata: std::collections::HashMap<String, serde_json::Value> = 
                    serde_json::from_str(row.get::<&str, _>("metadata")).unwrap_or_default();
                    
                let started_at: Option<String> = row.get("started_at");
                let started_at = started_at
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt: chrono::DateTime<chrono::FixedOffset>| dt.with_timezone(&chrono::Utc));
                    
                let last_updated: String = row.get("last_updated");
                let last_updated = chrono::DateTime::parse_from_rfc3339(&last_updated)
                    .map_err(|e| StoreError::SerializationError(e.to_string()))?
                    .with_timezone(&chrono::Utc);
                    
                Ok(Some(StreamInfo {
                    id: StreamId(row.get("id")),
                    platform: row.get("platform"),
                    user_id: row.get("user_id"),
                    display_name: row.get("display_name"),
                    avatar_url: row.get("avatar_url"),
                    is_live: row.get("is_live"),
                    title: row.get("title"),
                    viewer_count: row.get::<Option<i64>, _>("viewer_count").map(|v| v as u64),
                    thumbnail_url: row.get("thumbnail_url"),
                    category: row.get("category"),
                    tags,
                    language: row.get("language"),
                    started_at,
                    last_updated,
                    metadata,
                }))
            }
            None => Ok(None),
        }
    }

    async fn get_streams(&self, query: &StreamQuery) -> Result<StreamPage, StoreError> {
        trace!(?query, "Querying streams");
        
        // Build base query
        let mut sql = "SELECT * FROM streams WHERE 1=1".to_string();
        let mut where_clauses = Vec::new();
        
        // Add filters
        if let Some(ref platform) = query.platform {
            where_clauses.push("platform = ?");
        }
        
        if let Some(is_live) = query.is_live {
            where_clauses.push("is_live = ?");
        }
        
        if let Some(ref language) = query.language {
            where_clauses.push("language = ?");
        }
        
        if let Some(ref category) = query.category {
            where_clauses.push("category = ?");
        }
        
        if let Some(ref tag) = query.tag {
            where_clauses.push("tags LIKE ?");
        }
        
        if let Some(min_viewers) = query.min_viewers {
            where_clauses.push("viewer_count >= ?");
        }
        
        if !where_clauses.is_empty() {
            sql.push_str(" AND ");
            sql.push_str(&where_clauses.join(" AND "));
        }
        
        // Get total count
        let count_sql = format!("SELECT COUNT(*) as count FROM ({})", sql);
        let mut count_query = sqlx::query(&count_sql);
        
        // Bind parameters for count query
        if let Some(ref platform) = query.platform {
            count_query = count_query.bind(platform);
        }
        if let Some(is_live) = query.is_live {
            count_query = count_query.bind(is_live);
        }
        if let Some(ref language) = query.language {
            count_query = count_query.bind(language);
        }
        if let Some(ref category) = query.category {
            count_query = count_query.bind(category);
        }
        if let Some(ref tag) = query.tag {
            count_query = count_query.bind(format!("%\"{}\"%", tag));
        }
        if let Some(min_viewers) = query.min_viewers {
            count_query = count_query.bind(min_viewers as i64);
        }
        
        let total: i64 = count_query
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| StoreError::QueryError(e.to_string()))?
            .get("count");
            
        // Add ordering and pagination
        sql.push_str(" ORDER BY viewer_count DESC, display_name ASC");
        
        let page = query.page.unwrap_or(0);
        let page_size = query.page_size.unwrap_or(50).min(100);
        let offset = page * page_size;
        
        sql.push_str(" LIMIT ? OFFSET ?");
        
        // Execute main query
        let mut query_builder = sqlx::query(&sql);
        
        // Re-bind parameters for main query
        if let Some(ref platform) = query.platform {
            query_builder = query_builder.bind(platform);
        }
        if let Some(is_live) = query.is_live {
            query_builder = query_builder.bind(is_live);
        }
        if let Some(ref language) = query.language {
            query_builder = query_builder.bind(language);
        }
        if let Some(ref category) = query.category {
            query_builder = query_builder.bind(category);
        }
        if let Some(ref tag) = query.tag {
            query_builder = query_builder.bind(format!("%\"{}\"%", tag));
        }
        if let Some(min_viewers) = query.min_viewers {
            query_builder = query_builder.bind(min_viewers as i64);
        }
        
        // Add pagination bindings
        query_builder = query_builder.bind(page_size as i64);
        query_builder = query_builder.bind(offset as i64);
        
        let rows = query_builder
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| StoreError::QueryError(e.to_string()))?;
            
        // Convert rows to StreamInfo
        let mut items = Vec::new();
        for row in rows {
            let tags: Vec<String> = serde_json::from_str(
                row.get::<&str, _>("tags")
            ).unwrap_or_default();
            let metadata: std::collections::HashMap<String, serde_json::Value> = 
                serde_json::from_str(row.get::<&str, _>("metadata")).unwrap_or_default();
                
            let started_at: Option<String> = row.get("started_at");
            let started_at = started_at
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt: chrono::DateTime<chrono::FixedOffset>| dt.with_timezone(&chrono::Utc));
                
            let last_updated: String = row.get("last_updated");
            let last_updated = chrono::DateTime::parse_from_rfc3339(&last_updated)
                .map_err(|e| StoreError::SerializationError(e.to_string()))?
                .with_timezone(&chrono::Utc);
                
            items.push(StreamInfo {
                id: StreamId(row.get("id")),
                platform: row.get("platform"),
                user_id: row.get("user_id"),
                display_name: row.get("display_name"),
                avatar_url: row.get("avatar_url"),
                is_live: row.get("is_live"),
                title: row.get("title"),
                viewer_count: row.get::<Option<i64>, _>("viewer_count").map(|v| v as u64),
                thumbnail_url: row.get("thumbnail_url"),
                category: row.get("category"),
                tags,
                language: row.get("language"),
                started_at,
                last_updated,
                metadata,
            });
        }
        
        let total_pages = (total as usize + page_size - 1) / page_size;
        
        Ok(StreamPage {
            items,
            total: total as usize,
            page,
            page_size,
            total_pages,
        })
    }

    async fn delete_stream(&self, id: &StreamId) -> Result<(), StoreError> {
        trace!(stream_id = %id, "Deleting stream");
        
        sqlx::query("DELETE FROM streams WHERE id = ?")
            .bind(&id.0)
            .execute(&*self.pool)
            .await
            .map_err(|e| StoreError::QueryError(e.to_string()))?;
            
        Ok(())
    }

    async fn add_tracked_streamer(&self, streamer: &TrackedStreamer) -> Result<(), StoreError> {
        trace!(platform = %streamer.platform, user_id = %streamer.user_id, "Adding tracked streamer");
        
        let labels_json = serde_json::to_string(&streamer.labels)
            .map_err(|e| StoreError::SerializationError(e.to_string()))?;
            
        let result = sqlx::query(
            r#"
            INSERT INTO tracked_streamers (
                platform, user_id, custom_name, group_name, priority, labels, source, discovery_rule_id, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&streamer.platform)
        .bind(&streamer.user_id)
        .bind(&streamer.custom_name)
        .bind(&streamer.group)
        .bind(streamer.priority)
        .bind(labels_json)
        .bind(match streamer.source {
            StreamerSource::Manual => "manual",
            StreamerSource::Discovery => "discovery",
        })
        .bind(&streamer.discovery_rule_id.as_deref())
        .bind(streamer.created_at.to_rfc3339())
        .execute(&*self.pool)
        .await;
            
        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(ref err)) if err.message().contains("UNIQUE constraint failed") => {
                Err(StoreError::AlreadyExists(format!(
                    "Streamer {} on {} is already tracked",
                    streamer.user_id, streamer.platform
                )))
            }
            Err(e) => Err(StoreError::QueryError(e.to_string())),
        }
    }

    async fn get_tracked_streamer(
        &self,
        platform: &str,
        user_id: &str,
    ) -> Result<Option<TrackedStreamer>, StoreError> {
        trace!(platform = %platform, user_id = %user_id, "Getting tracked streamer");
        
        let row = sqlx::query("SELECT * FROM tracked_streamers WHERE platform = ? AND user_id = ?")
            .bind(platform)
            .bind(user_id)
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| StoreError::QueryError(e.to_string()))?;
            
        match row {
            Some(row) => {
                let labels: std::collections::HashMap<String, String> = serde_json::from_str(
                    row.get::<&str, _>("labels")
                ).unwrap_or_default();
                
                let source: String = row.get("source");
                let source = match source.as_str() {
                    "manual" => StreamerSource::Manual,
                    "discovery" => StreamerSource::Discovery,
                    _ => StreamerSource::Manual,
                };
                
                let created_at: String = row.get("created_at");
                let created_at = chrono::DateTime::parse_from_rfc3339(&created_at)
                    .map_err(|e| StoreError::SerializationError(e.to_string()))?
                    .with_timezone(&chrono::Utc);
                
                Ok(Some(TrackedStreamer {
                    platform: row.get("platform"),
                    user_id: row.get("user_id"),
                    custom_name: row.get("custom_name"),
                    group: row.get("group_name"),
                    priority: row.get("priority"),
                    labels,
                    source,
                    discovery_rule_id: row.get("discovery_rule_id"),
                    created_at,
                }))
            }
            None => Ok(None),
        }
    }

    async fn get_tracked_streamers(
        &self,
        query: &TrackedStreamerQuery,
    ) -> Result<Vec<TrackedStreamer>, StoreError> {
        trace!(?query, "Querying tracked streamers");
        
        let mut sql = "SELECT * FROM tracked_streamers WHERE 1=1".to_string();
        
        if let Some(ref platform) = query.platform {
            sql.push_str(" AND platform = ?");
        }
        
        if let Some(ref group) = query.group {
            sql.push_str(" AND group_name = ?");
        }
        
        if let Some(ref source) = query.source {
            sql.push_str(" AND source = ?");
        }
        
        // Handle labels filter
        for (key, value) in &query.labels {
            sql.push_str(" AND labels LIKE ?");
        }
        
        sql.push_str(" ORDER BY platform, user_id");
        
        let mut query_builder = sqlx::query(&sql);
        
        if let Some(ref platform) = query.platform {
            query_builder = query_builder.bind(platform);
        }
        if let Some(ref group) = query.group {
            query_builder = query_builder.bind(group);
        }
        if let Some(ref source) = query.source {
            let source_str = match source {
                StreamerSource::Manual => "manual",
                StreamerSource::Discovery => "discovery",
            };
            query_builder = query_builder.bind(source_str);
        }
        
        // Bind label filters
        for (key, value) in &query.labels {
            query_builder = query_builder.bind(format!("%\"{}\":\"{}\"%", key, value));
        }
        
        let rows = query_builder
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| StoreError::QueryError(e.to_string()))?;
            
        let mut streamers = Vec::new();
        for row in rows {
            let labels: std::collections::HashMap<String, String> = serde_json::from_str(
                row.get::<&str, _>("labels")
            ).unwrap_or_default();
            
            let source: String = row.get("source");
            let source = match source.as_str() {
                "manual" => StreamerSource::Manual,
                "discovery" => StreamerSource::Discovery,
                _ => StreamerSource::Manual,
            };
            
            let created_at: String = row.get("created_at");
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at)
                .map_err(|e| StoreError::SerializationError(e.to_string()))?
                .with_timezone(&chrono::Utc);
            
            streamers.push(TrackedStreamer {
                platform: row.get("platform"),
                user_id: row.get("user_id"),
                custom_name: row.get("custom_name"),
                group: row.get("group_name"),
                priority: row.get("priority"),
                labels,
                source,
                discovery_rule_id: row.get("discovery_rule_id"),
                created_at,
            });
        }
        
        Ok(streamers)
    }

    async fn remove_tracked_streamer(&self, platform: &str, user_id: &str) -> Result<(), StoreError> {
        trace!(platform = %platform, user_id = %user_id, "Removing tracked streamer");
        
        sqlx::query("DELETE FROM tracked_streamers WHERE platform = ? AND user_id = ?")
            .bind(platform)
            .bind(user_id)
            .execute(&*self.pool)
            .await
            .map_err(|e| StoreError::QueryError(e.to_string()))?;
            
        Ok(())
    }

    async fn update_tracked_streamer(&self, streamer: &TrackedStreamer) -> Result<(), StoreError> {
        trace!(platform = %streamer.platform, user_id = %streamer.user_id, "Updating tracked streamer");
        
        let labels_json = serde_json::to_string(&streamer.labels)
            .map_err(|e| StoreError::SerializationError(e.to_string()))?;
            
        let result = sqlx::query(
            r#"
            UPDATE tracked_streamers SET
                custom_name = ?, group_name = ?, priority = ?, labels = ?, source = ?, discovery_rule_id = ?
            WHERE platform = ? AND user_id = ?
            "#,
        )
        .bind(&streamer.custom_name)
        .bind(&streamer.group)
        .bind(streamer.priority)
        .bind(labels_json)
        .bind(match streamer.source {
            StreamerSource::Manual => "manual",
            StreamerSource::Discovery => "discovery",
        })
        .bind(streamer.discovery_rule_id.as_deref())
        .bind(&streamer.platform)
        .bind(&streamer.user_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| StoreError::QueryError(e.to_string()))?;
            
        if result.rows_affected() == 0 {
            return Err(StoreError::NotFound(format!(
                "Streamer {} on {} not found",
                streamer.user_id, streamer.platform
            )));
        }
        
        Ok(())
    }

    async fn add_discovery_rule(&self, rule: &DiscoveryRule) -> Result<(), StoreError> {
        trace!(rule_id = %rule.id, "Adding discovery rule");
        
        let filters_json = serde_json::to_string(&rule.filters)
            .map_err(|e| StoreError::SerializationError(e.to_string()))?;
        let apply_labels_json = serde_json::to_string(&rule.apply_labels)
            .map_err(|e| StoreError::SerializationError(e.to_string()))?;
            
        let result = sqlx::query(
            r#"
            INSERT INTO discovery_rules (
                id, name, platform, enabled, filters, interval_secs, apply_labels, apply_group, created_at, last_run_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&rule.id)
        .bind(&rule.name)
        .bind(&rule.platform)
        .bind(rule.enabled)
        .bind(filters_json)
        .bind(rule.interval_secs as i64)
        .bind(apply_labels_json)
        .bind(&rule.apply_group)
        .bind(rule.created_at.to_rfc3339())
        .bind(rule.last_run_at.map(|dt| dt.to_rfc3339()))
        .execute(&*self.pool)
        .await;
            
        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(ref err)) if err.message().contains("UNIQUE constraint failed") => {
                Err(StoreError::AlreadyExists(format!(
                    "Discovery rule {} already exists",
                    rule.id
                )))
            }
            Err(e) => Err(StoreError::QueryError(e.to_string())),
        }
    }

    async fn get_discovery_rule(&self, id: &str) -> Result<Option<DiscoveryRule>, StoreError> {
        trace!(rule_id = %id, "Getting discovery rule");
        
        let row = sqlx::query("SELECT * FROM discovery_rules WHERE id = ?")
            .bind(id)
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| StoreError::QueryError(e.to_string()))?;
            
        match row {
            Some(row) => {
                let filters: DiscoveryFilters = serde_json::from_str(
                    row.get::<&str, _>("filters")
                ).unwrap_or_default();
                let apply_labels: std::collections::HashMap<String, String> = serde_json::from_str(
                    row.get::<&str, _>("apply_labels")
                ).unwrap_or_default();
                
                let last_run_at: Option<String> = row.get("last_run_at");
                let last_run_at = last_run_at
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt: chrono::DateTime<chrono::FixedOffset>| dt.with_timezone(&chrono::Utc));
                
                let created_at: String = row.get("created_at");
                let created_at = chrono::DateTime::parse_from_rfc3339(&created_at)
                    .map_err(|e| StoreError::SerializationError(e.to_string()))?
                    .with_timezone(&chrono::Utc);
                
                Ok(Some(DiscoveryRule {
                    id: row.get("id"),
                    name: row.get("name"),
                    platform: row.get("platform"),
                    enabled: row.get("enabled"),
                    filters,
                    interval_secs: row.get::<i64, _>("interval_secs") as u64,
                    apply_labels,
                    apply_group: row.get("apply_group"),
                    created_at,
                    last_run_at,
                }))
            }
            None => Ok(None),
        }
    }

    async fn get_discovery_rules(&self, platform: Option<&str>) -> Result<Vec<DiscoveryRule>, StoreError> {
        trace!(?platform, "Querying discovery rules");
        
        let (sql, binding) = match platform {
            Some(p) => ("SELECT * FROM discovery_rules WHERE platform = ? ORDER BY created_at", Some(p)),
            None => ("SELECT * FROM discovery_rules ORDER BY platform, created_at", None),
        };
        
        let rows = if let Some(p) = binding {
            sqlx::query(sql).bind(p)
        } else {
            sqlx::query(sql)
        }
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| StoreError::QueryError(e.to_string()))?;
            
        let mut rules = Vec::new();
        for row in rows {
            let filters: DiscoveryFilters = serde_json::from_str(
                row.get::<&str, _>("filters")
            ).unwrap_or_default();
            let apply_labels: std::collections::HashMap<String, String> = serde_json::from_str(
                row.get::<&str, _>("apply_labels")
            ).unwrap_or_default();
            
            let last_run_at: Option<String> = row.get("last_run_at");
            let last_run_at = last_run_at
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt: chrono::DateTime<chrono::FixedOffset>| dt.with_timezone(&chrono::Utc));
            
            let created_at: String = row.get("created_at");
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at)
                .map_err(|e| StoreError::SerializationError(e.to_string()))?
                .with_timezone(&chrono::Utc);
            
            rules.push(DiscoveryRule {
                id: row.get("id"),
                name: row.get("name"),
                platform: row.get("platform"),
                enabled: row.get("enabled"),
                filters,
                interval_secs: row.get::<i64, _>("interval_secs") as u64,
                apply_labels,
                apply_group: row.get("apply_group"),
                created_at,
                last_run_at,
            });
        }
        
        Ok(rules)
    }

    async fn update_discovery_rule(&self, rule: &DiscoveryRule) -> Result<(), StoreError> {
        trace!(rule_id = %rule.id, "Updating discovery rule");
        
        let filters_json = serde_json::to_string(&rule.filters)
            .map_err(|e| StoreError::SerializationError(e.to_string()))?;
        let apply_labels_json = serde_json::to_string(&rule.apply_labels)
            .map_err(|e| StoreError::SerializationError(e.to_string()))?;
            
        let result = sqlx::query(
            r#"
            UPDATE discovery_rules SET
                name = ?, platform = ?, enabled = ?, filters = ?, interval_secs = ?, 
                apply_labels = ?, apply_group = ?, last_run_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&rule.name)
        .bind(&rule.platform)
        .bind(rule.enabled)
        .bind(filters_json)
        .bind(rule.interval_secs as i64)
        .bind(apply_labels_json)
        .bind(&rule.apply_group)
        .bind(rule.last_run_at.map(|dt| dt.to_rfc3339()))
        .bind(&rule.id)
        .execute(&*self.pool)
        .await
        .map_err(|e| StoreError::QueryError(e.to_string()))?;
            
        if result.rows_affected() == 0 {
            return Err(StoreError::NotFound(format!(
                "Discovery rule {} not found",
                rule.id
            )));
        }
        
        Ok(())
    }

    async fn remove_discovery_rule(&self, id: &str) -> Result<(), StoreError> {
        trace!(rule_id = %id, "Removing discovery rule");
        
        sqlx::query("DELETE FROM discovery_rules WHERE id = ?")
            .bind(id)
            .execute(&*self.pool)
            .await
            .map_err(|e| StoreError::QueryError(e.to_string()))?;
            
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_sqlite_stream_operations() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = SqliteStore::new(db_path.to_str().unwrap()).await.unwrap();

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
    async fn test_sqlite_tracked_streamer_operations() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = SqliteStore::new(db_path.to_str().unwrap()).await.unwrap();

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
        store.remove_tracked_streamer("twitch", "user123").await.unwrap();
        let deleted = store
            .get_tracked_streamer("twitch", "user123")
            .await
            .unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_sqlite_discovery_rule_operations() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = SqliteStore::new(db_path.to_str().unwrap()).await.unwrap();

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