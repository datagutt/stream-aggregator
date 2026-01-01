//! Diesel-based SQLite storage implementation

use async_trait::async_trait;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, CustomizeConnection, Pool};
use diesel::sqlite::SqliteConnection;
use std::sync::Arc;
use tracing::{debug, trace};

use stream_aggregator_core::{errors::StoreError, models::*, traits::StreamStore};

use crate::models::*;
use crate::schema::{discovery_rules, streams, tracked_streamers};

type DbPool = Pool<ConnectionManager<SqliteConnection>>;

/// SQLite connection customizer to enable WAL mode and set busy timeout
#[derive(Debug, Clone, Copy)]
struct SqliteConnectionCustomizer;

impl CustomizeConnection<SqliteConnection, diesel::r2d2::Error> for SqliteConnectionCustomizer {
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), diesel::r2d2::Error> {
        use diesel::sql_query;

        // Enable WAL mode for better concurrency
        sql_query("PRAGMA journal_mode = WAL")
            .execute(conn)
            .map_err(diesel::r2d2::Error::QueryError)?;

        // Set busy timeout to 30 seconds to handle concurrent writes
        sql_query("PRAGMA busy_timeout = 30000")
            .execute(conn)
            .map_err(diesel::r2d2::Error::QueryError)?;

        // Enable foreign keys
        sql_query("PRAGMA foreign_keys = ON")
            .execute(conn)
            .map_err(diesel::r2d2::Error::QueryError)?;

        // Optimize for concurrent writes
        sql_query("PRAGMA synchronous = NORMAL")
            .execute(conn)
            .map_err(diesel::r2d2::Error::QueryError)?;

        Ok(())
    }
}

/// Diesel-based SQLite storage implementation
///
/// Uses Diesel ORM with r2d2 connection pooling for thread-safe, async operations.
#[derive(Clone)]
pub struct DieselStore {
    pool: Arc<DbPool>,
}

impl DieselStore {
    /// Create a new DieselStore with the given database URL
    ///
    /// # Arguments
    /// * `database_url` - Path to SQLite database file or ":memory:" for in-memory
    pub fn new(database_url: &str) -> Result<Self, StoreError> {
        debug!("Creating new DieselStore with database: {}", database_url);

        let manager = ConnectionManager::<SqliteConnection>::new(database_url);
        let pool = Pool::builder()
            .max_size(20) // Increase pool size for concurrent writes
            .connection_timeout(std::time::Duration::from_secs(30))
            .connection_customizer(Box::new(SqliteConnectionCustomizer))
            .build(manager)
            .map_err(|e| StoreError::ConnectionError(e.to_string()))?;

        let store = Self {
            pool: Arc::new(pool),
        };

        // Run migrations
        store.run_migrations()?;

        debug!("DieselStore initialized successfully");
        Ok(store)
    }

    /// Create an in-memory database instance
    pub fn memory() -> Result<Self, StoreError> {
        Self::new(":memory:")
    }

    /// Run database migrations
    fn run_migrations(&self) -> Result<(), StoreError> {
        use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

        const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

        let mut conn = self
            .pool
            .get()
            .map_err(|e| StoreError::ConnectionError(e.to_string()))?;

        conn.run_pending_migrations(MIGRATIONS)
            .map_err(|e| StoreError::QueryError(format!("Migration failed: {}", e)))?;

        debug!("Diesel migrations completed successfully");
        Ok(())
    }

    /// Get a connection from the pool (sync operation for blocking contexts)
    #[allow(dead_code)]
    fn get_conn(
        &self,
    ) -> Result<r2d2::PooledConnection<ConnectionManager<SqliteConnection>>, StoreError> {
        self.pool
            .get()
            .map_err(|e| StoreError::ConnectionError(e.to_string()))
    }
}

#[async_trait]
impl StreamStore for DieselStore {
    async fn upsert_stream(&self, stream: &StreamInfo) -> Result<(), StoreError> {
        trace!(stream_id = %stream.id, platform = %stream.platform, "Upserting stream");

        // Clone stream data to move into spawn_blocking
        let stream = stream.clone();
        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| StoreError::ConnectionError(e.to_string()))?;

            let new_stream = NewStream::from_stream_info(&stream)
                .map_err(|e| StoreError::SerializationError(e.to_string()))?;

            diesel::replace_into(streams::table)
                .values(&new_stream)
                .execute(&mut conn)
                .map_err(|e| StoreError::QueryError(e.to_string()))?;

            Ok::<(), StoreError>(())
        })
        .await
        .map_err(|e| StoreError::QueryError(format!("Task join error: {}", e)))??;

        Ok(())
    }

    async fn batch_upsert_streams(&self, streams: &[StreamInfo]) -> Result<(), StoreError> {
        if streams.is_empty() {
            return Ok(());
        }

        let stream_count = streams.len();
        debug!("Batch upserting {} streams", stream_count);

        let streams = streams.to_vec();
        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| StoreError::ConnectionError(e.to_string()))?;

            // Use a transaction for atomicity and performance
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                // SQLite has a limit of 999 parameters per query
                // Each stream has ~15 fields, so we chunk to stay well under the limit
                const CHUNK_SIZE: usize = 50;

                for chunk in streams.chunks(CHUNK_SIZE) {
                    let new_streams: Vec<NewStream> = chunk
                        .iter()
                        .map(|s| NewStream::from_stream_info(s))
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| diesel::result::Error::DeserializationError(Box::new(e)))?;

                    diesel::replace_into(streams::table)
                        .values(&new_streams)
                        .execute(conn)?;
                }

                Ok(())
            })
            .map_err(|e| StoreError::QueryError(e.to_string()))
        })
        .await
        .map_err(|e| StoreError::QueryError(format!("Task join error: {}", e)))??;

        debug!("Successfully batch upserted {} streams", stream_count);
        Ok(())
    }

    async fn get_stream(&self, id: &StreamId) -> Result<Option<StreamInfo>, StoreError> {
        trace!(stream_id = %id, "Getting stream");

        let stream_id = id.0.clone();
        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| StoreError::ConnectionError(e.to_string()))?;

            let result: Option<StreamRow> = streams::table
                .find(&stream_id)
                .first::<StreamRow>(&mut conn)
                .optional()
                .map_err(|e| StoreError::QueryError(e.to_string()))?;

            match result {
                Some(row) => {
                    let stream = row
                        .to_stream_info()
                        .map_err(|e| StoreError::SerializationError(e.to_string()))?;
                    Ok(Some(stream))
                }
                None => Ok(None),
            }
        })
        .await
        .map_err(|e| StoreError::QueryError(format!("Task join error: {}", e)))?
    }

    async fn get_streams(&self, query: &StreamQuery) -> Result<StreamPage, StoreError> {
        trace!(?query, "Querying streams");

        let query = query.clone();
        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| StoreError::ConnectionError(e.to_string()))?;

            // Macro to apply filters (avoids lifetime issues)
            macro_rules! apply_filters {
                ($diesel_query:expr) => {{
                    let mut dq = $diesel_query;

                    if let Some(ref platform) = query.platform {
                        dq = dq.filter(streams::platform.eq(platform.clone()));
                    }

                    if let Some(is_live) = query.is_live {
                        dq = dq.filter(streams::is_live.eq(is_live));
                    }

                    if let Some(ref language) = query.language {
                        dq = dq.filter(streams::language.eq(language.clone()));
                    }

                    if let Some(ref category) = query.category {
                        dq = dq.filter(streams::category.eq(category.clone()));
                    }

                    if let Some(min_viewers) = query.min_viewers {
                        dq = dq.filter(streams::viewer_count.ge(min_viewers as i32));
                    }

                    if let Some(max_viewers) = query.max_viewers {
                        dq = dq.filter(streams::viewer_count.le(max_viewers as i32));
                    }

                    if let Some(ref search) = query.search {
                        let pattern = format!("%{}%", search);
                        dq = dq.filter(
                            streams::display_name
                                .like(pattern.clone())
                                .or(streams::title.like(pattern)),
                        );
                    }

                    if let Some(ref tag) = query.tag {
                        let pattern = format!("%\"{}%", tag);
                        dq = dq.filter(streams::tags.like(pattern));
                    }

                    dq
                }};
            }

            // Build count query
            let count_query = apply_filters!(streams::table.into_boxed());
            let total = count_query
                .count()
                .get_result::<i64>(&mut conn)
                .map_err(|e| StoreError::QueryError(e.to_string()))?
                as usize;

            // Build data query (rebuild from scratch since count consumed the first one)
            let mut data_query = apply_filters!(streams::table.into_boxed());

            // Apply sorting
            let sort_field = query.sort.as_deref().unwrap_or("viewers");
            let is_ascending = query.order.as_deref().unwrap_or("desc") == "asc";

            data_query = match (sort_field, is_ascending) {
                ("name", true) => data_query.order(streams::display_name.asc()),
                ("name", false) => data_query.order(streams::display_name.desc()),
                ("platform", true) => data_query.order(streams::platform.asc()),
                ("platform", false) => data_query.order(streams::platform.desc()),
                ("updated", true) => data_query.order(streams::last_updated.asc()),
                ("updated", false) => data_query.order(streams::last_updated.desc()),
                ("viewers", true) | (_, true) => data_query
                    .order(streams::viewer_count.asc())
                    .then_order_by(streams::display_name.asc()),
                ("viewers", false) | (_, false) => data_query
                    .order(streams::viewer_count.desc())
                    .then_order_by(streams::display_name.asc()),
            };

            // Apply pagination at SQL level
            let page = query.page.unwrap_or(0);
            let page_size = query.page_size.unwrap_or(50).min(100);
            let offset = (page * page_size) as i64;

            data_query = data_query.limit(page_size as i64).offset(offset);

            // Execute query
            let rows: Vec<StreamRow> = data_query
                .load::<StreamRow>(&mut conn)
                .map_err(|e| StoreError::QueryError(e.to_string()))?;

            // Convert rows to StreamInfo
            let mut items = Vec::new();
            for row in rows {
                let stream = row
                    .to_stream_info()
                    .map_err(|e| StoreError::SerializationError(e.to_string()))?;
                items.push(stream);
            }

            let total_pages = total.div_ceil(page_size);

            Ok(StreamPage {
                items,
                total,
                page,
                page_size,
                total_pages,
            })
        })
        .await
        .map_err(|e| StoreError::QueryError(format!("Task join error: {}", e)))?
    }

    async fn delete_stream(&self, id: &StreamId) -> Result<(), StoreError> {
        trace!(stream_id = %id, "Deleting stream");

        let stream_id = id.0.clone();
        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| StoreError::ConnectionError(e.to_string()))?;

            diesel::delete(streams::table.find(&stream_id))
                .execute(&mut conn)
                .map_err(|e| StoreError::QueryError(e.to_string()))?;

            Ok(())
        })
        .await
        .map_err(|e| StoreError::QueryError(format!("Task join error: {}", e)))?
    }

    async fn add_tracked_streamer(&self, streamer: &TrackedStreamer) -> Result<(), StoreError> {
        trace!(platform = %streamer.platform, user_id = %streamer.user_id, "Adding tracked streamer");

        let streamer = streamer.clone();
        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let new_streamer = NewTrackedStreamer::from_tracked_streamer(&streamer)
                .map_err(|e| StoreError::SerializationError(e.to_string()))?;
            let mut conn = pool
                .get()
                .map_err(|e| StoreError::ConnectionError(e.to_string()))?;

            let result = diesel::insert_into(tracked_streamers::table)
                .values(&new_streamer)
                .execute(&mut conn);

            match result {
                Ok(_) => Ok(()),
                Err(diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    _,
                )) => Err(StoreError::AlreadyExists(format!(
                    "Streamer {} on {} is already tracked",
                    new_streamer.user_id, new_streamer.platform
                ))),
                Err(e) => Err(StoreError::QueryError(e.to_string())),
            }
        })
        .await
        .map_err(|e| StoreError::QueryError(format!("Task join error: {}", e)))?
    }

    async fn get_tracked_streamer(
        &self,
        platform: &str,
        user_id: &str,
    ) -> Result<Option<TrackedStreamer>, StoreError> {
        trace!(platform = %platform, user_id = %user_id, "Getting tracked streamer");

        let platform = platform.to_string();
        let user_id = user_id.to_string();
        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| StoreError::ConnectionError(e.to_string()))?;

            let result: Option<TrackedStreamerRow> = tracked_streamers::table
                .find((&platform, &user_id))
                .first::<TrackedStreamerRow>(&mut conn)
                .optional()
                .map_err(|e| StoreError::QueryError(e.to_string()))?;

            match result {
                Some(row) => {
                    let streamer = row
                        .to_tracked_streamer()
                        .map_err(|e| StoreError::SerializationError(e.to_string()))?;
                    Ok(Some(streamer))
                }
                None => Ok(None),
            }
        })
        .await
        .map_err(|e| StoreError::QueryError(format!("Task join error: {}", e)))?
    }

    async fn get_tracked_streamers(
        &self,
        query: &TrackedStreamerQuery,
    ) -> Result<Vec<TrackedStreamer>, StoreError> {
        trace!(?query, "Querying tracked streamers");

        let query = query.clone();
        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| StoreError::ConnectionError(e.to_string()))?;

            let mut diesel_query = tracked_streamers::table.into_boxed();

            if let Some(ref platform) = query.platform {
                diesel_query = diesel_query.filter(tracked_streamers::platform.eq(platform));
            }

            if let Some(ref group) = query.group {
                diesel_query = diesel_query.filter(tracked_streamers::group_name.eq(group));
            }

            if let Some(ref source) = query.source {
                let source_str = match source {
                    StreamerSource::Manual => "manual",
                    StreamerSource::Discovery => "discovery",
                };
                diesel_query = diesel_query.filter(tracked_streamers::source.eq(source_str));
            }

            diesel_query = diesel_query.order((
                tracked_streamers::platform.asc(),
                tracked_streamers::user_id.asc(),
            ));

            let rows: Vec<TrackedStreamerRow> = diesel_query
                .load::<TrackedStreamerRow>(&mut conn)
                .map_err(|e| StoreError::QueryError(e.to_string()))?;

            let mut streamers = Vec::new();
            for row in rows {
                let streamer = row
                    .to_tracked_streamer()
                    .map_err(|e| StoreError::SerializationError(e.to_string()))?;
                streamers.push(streamer);
            }

            Ok(streamers)
        })
        .await
        .map_err(|e| StoreError::QueryError(format!("Task join error: {}", e)))?
    }

    async fn remove_tracked_streamer(
        &self,
        platform: &str,
        user_id: &str,
    ) -> Result<(), StoreError> {
        trace!(platform = %platform, user_id = %user_id, "Removing tracked streamer");

        let platform = platform.to_string();
        let user_id = user_id.to_string();
        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| StoreError::ConnectionError(e.to_string()))?;

            diesel::delete(tracked_streamers::table.find((&platform, &user_id)))
                .execute(&mut conn)
                .map_err(|e| StoreError::QueryError(e.to_string()))?;

            Ok(())
        })
        .await
        .map_err(|e| StoreError::QueryError(format!("Task join error: {}", e)))?
    }

    async fn update_tracked_streamer(&self, streamer: &TrackedStreamer) -> Result<(), StoreError> {
        trace!(platform = %streamer.platform, user_id = %streamer.user_id, "Updating tracked streamer");

        let streamer = streamer.clone();
        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let update = UpdateTrackedStreamer::from_tracked_streamer(&streamer)
                .map_err(|e| StoreError::SerializationError(e.to_string()))?;

            let platform = &streamer.platform;
            let user_id = &streamer.user_id;
            let mut conn = pool
                .get()
                .map_err(|e| StoreError::ConnectionError(e.to_string()))?;

            let rows_affected =
                diesel::update(tracked_streamers::table.find((&platform, &user_id)))
                    .set(&update)
                    .execute(&mut conn)
                    .map_err(|e| StoreError::QueryError(e.to_string()))?;

            if rows_affected == 0 {
                return Err(StoreError::NotFound(format!(
                    "Streamer {} on {} not found",
                    user_id, platform
                )));
            }

            Ok(())
        })
        .await
        .map_err(|e| StoreError::QueryError(format!("Task join error: {}", e)))?
    }

    async fn add_discovery_rule(&self, rule: &DiscoveryRule) -> Result<(), StoreError> {
        trace!(rule_id = %rule.id, "Adding discovery rule");

        let rule = rule.clone();
        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let new_rule = NewDiscoveryRule::from_discovery_rule(&rule)
                .map_err(|e| StoreError::SerializationError(e.to_string()))?;
            let mut conn = pool
                .get()
                .map_err(|e| StoreError::ConnectionError(e.to_string()))?;

            let result = diesel::insert_into(discovery_rules::table)
                .values(&new_rule)
                .execute(&mut conn);

            match result {
                Ok(_) => Ok(()),
                Err(diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    _,
                )) => Err(StoreError::AlreadyExists(format!(
                    "Discovery rule {} already exists",
                    new_rule.id
                ))),
                Err(e) => Err(StoreError::QueryError(e.to_string())),
            }
        })
        .await
        .map_err(|e| StoreError::QueryError(format!("Task join error: {}", e)))?
    }

    async fn get_discovery_rule(&self, id: &str) -> Result<Option<DiscoveryRule>, StoreError> {
        trace!(rule_id = %id, "Getting discovery rule");

        let id = id.to_string();
        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| StoreError::ConnectionError(e.to_string()))?;

            let result: Option<DiscoveryRuleRow> = discovery_rules::table
                .find(&id)
                .first::<DiscoveryRuleRow>(&mut conn)
                .optional()
                .map_err(|e| StoreError::QueryError(e.to_string()))?;

            match result {
                Some(row) => {
                    let rule = row
                        .to_discovery_rule()
                        .map_err(|e| StoreError::SerializationError(e.to_string()))?;
                    Ok(Some(rule))
                }
                None => Ok(None),
            }
        })
        .await
        .map_err(|e| StoreError::QueryError(format!("Task join error: {}", e)))?
    }

    async fn get_discovery_rules(
        &self,
        platform: Option<&str>,
    ) -> Result<Vec<DiscoveryRule>, StoreError> {
        trace!(?platform, "Querying discovery rules");

        let platform = platform.map(|s| s.to_string());
        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| StoreError::ConnectionError(e.to_string()))?;

            let mut diesel_query = discovery_rules::table.into_boxed();

            if let Some(ref platform) = platform {
                diesel_query = diesel_query.filter(discovery_rules::platform.eq(platform));
            }

            diesel_query = diesel_query.order(discovery_rules::created_at.asc());

            let rows: Vec<DiscoveryRuleRow> = diesel_query
                .load::<DiscoveryRuleRow>(&mut conn)
                .map_err(|e| StoreError::QueryError(e.to_string()))?;

            let mut rules = Vec::new();
            for row in rows {
                let rule = row
                    .to_discovery_rule()
                    .map_err(|e| StoreError::SerializationError(e.to_string()))?;
                rules.push(rule);
            }

            Ok(rules)
        })
        .await
        .map_err(|e| StoreError::QueryError(format!("Task join error: {}", e)))?
    }

    async fn update_discovery_rule(&self, rule: &DiscoveryRule) -> Result<(), StoreError> {
        trace!(rule_id = %rule.id, "Updating discovery rule");

        let rule = rule.clone();
        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let update = UpdateDiscoveryRule::from_discovery_rule(&rule)
                .map_err(|e| StoreError::SerializationError(e.to_string()))?;

            let id = &rule.id;
            let mut conn = pool
                .get()
                .map_err(|e| StoreError::ConnectionError(e.to_string()))?;

            let rows_affected = diesel::update(discovery_rules::table.find(&id))
                .set(&update)
                .execute(&mut conn)
                .map_err(|e| StoreError::QueryError(e.to_string()))?;

            if rows_affected == 0 {
                return Err(StoreError::NotFound(format!(
                    "Discovery rule {} not found",
                    id
                )));
            }

            Ok(())
        })
        .await
        .map_err(|e| StoreError::QueryError(format!("Task join error: {}", e)))?
    }

    async fn remove_discovery_rule(&self, id: &str) -> Result<(), StoreError> {
        trace!(rule_id = %id, "Removing discovery rule");

        let id = id.to_string();
        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| StoreError::ConnectionError(e.to_string()))?;

            diesel::delete(discovery_rules::table.find(&id))
                .execute(&mut conn)
                .map_err(|e| StoreError::QueryError(e.to_string()))?;

            Ok(())
        })
        .await
        .map_err(|e| StoreError::QueryError(format!("Task join error: {}", e)))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_diesel_stream_operations() {
        let store = DieselStore::memory().unwrap();

        let stream = StreamInfo::new("twitch", "user123", "TestUser");
        let id = stream.id.clone();

        store.upsert_stream(&stream).await.unwrap();

        let retrieved = store.get_stream(&id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().display_name, "TestUser");

        store.delete_stream(&id).await.unwrap();
        let deleted = store.get_stream(&id).await.unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_diesel_tracked_streamer_operations() {
        let store = DieselStore::memory().unwrap();

        let streamer = TrackedStreamer::new_manual("twitch", "user123");
        store.add_tracked_streamer(&streamer).await.unwrap();

        let retrieved = store
            .get_tracked_streamer("twitch", "user123")
            .await
            .unwrap();
        assert!(retrieved.is_some());

        let result = store.add_tracked_streamer(&streamer).await;
        assert!(result.is_err());

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
    async fn test_diesel_discovery_rule_operations() {
        let store = DieselStore::memory().unwrap();

        let rule = DiscoveryRule::new("rule1", "Test Rule", "twitch");
        store.add_discovery_rule(&rule).await.unwrap();

        let retrieved = store.get_discovery_rule("rule1").await.unwrap();
        assert!(retrieved.is_some());

        let rules = store.get_discovery_rules(None).await.unwrap();
        assert_eq!(rules.len(), 1);

        store.remove_discovery_rule("rule1").await.unwrap();
        let deleted = store.get_discovery_rule("rule1").await.unwrap();
        assert!(deleted.is_none());
    }
}
