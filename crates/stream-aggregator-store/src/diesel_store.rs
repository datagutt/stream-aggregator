//! Diesel-based SQLite storage implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, CustomizeConnection, Pool};
use diesel::sqlite::SqliteConnection;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, trace};

use stream_aggregator_core::{errors::StoreError, models::*, traits::StreamStore};

use crate::models::*;
use crate::schema::{communities, community_domains, discovery_rules, streams, tracked_streamers};

type DbPool = Pool<ConnectionManager<SqliteConnection>>;

fn parse_rfc3339_opt(s: Option<&str>) -> Option<DateTime<Utc>> {
    s.and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
        .map(|dt| dt.with_timezone(&Utc))
}

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

        let stream = stream.clone();
        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| StoreError::ConnectionError(e.to_string()))?;

            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let prior: Option<(bool, Option<String>)> = streams::table
                    .find(&stream.id.0)
                    .select((streams::is_live, streams::last_live_at))
                    .first::<(bool, Option<String>)>(conn)
                    .optional()?;

                let (prior_was_live, prior_last_live) = match prior {
                    Some((live, ts)) => (live, parse_rfc3339_opt(ts.as_deref())),
                    None => (false, None),
                };

                let mut merged = stream.clone();
                merged.last_live_at = StreamInfo::merge_last_live_at(
                    merged.is_live,
                    merged.started_at,
                    prior_last_live,
                    prior_was_live,
                    Utc::now(),
                );

                let new_stream = NewStream::from_stream_info(&merged)
                    .map_err(|e| diesel::result::Error::DeserializationError(Box::new(e)))?;

                diesel::replace_into(streams::table)
                    .values(&new_stream)
                    .execute(conn)?;

                Ok(())
            })
            .map_err(|e| StoreError::QueryError(e.to_string()))
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

            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                // SQLite has a limit of 999 parameters per query. Each stream has
                // ~16 fields, so we chunk to stay well under the limit.
                const CHUNK_SIZE: usize = 50;

                for chunk in streams.chunks(CHUNK_SIZE) {
                    let ids: Vec<&str> = chunk.iter().map(|s| s.id.0.as_str()).collect();

                    // Read prior (is_live, last_live_at) for all rows in this chunk
                    let prior_rows: Vec<(String, bool, Option<String>)> = streams::table
                        .select((streams::id, streams::is_live, streams::last_live_at))
                        .filter(streams::id.eq_any(&ids))
                        .load(conn)?;

                    let prior_by_id: HashMap<String, (bool, Option<String>)> = prior_rows
                        .into_iter()
                        .map(|(id, live, ts)| (id, (live, ts)))
                        .collect();

                    let now = Utc::now();
                    let merged_chunk: Vec<StreamInfo> = chunk
                        .iter()
                        .map(|s| {
                            let (prior_was_live, prior_last_live) = prior_by_id
                                .get(&s.id.0)
                                .map(|(live, ts)| {
                                    (*live, parse_rfc3339_opt(ts.as_deref()))
                                })
                                .unwrap_or((false, None));

                            let mut merged = s.clone();
                            merged.last_live_at = StreamInfo::merge_last_live_at(
                                merged.is_live,
                                merged.started_at,
                                prior_last_live,
                                prior_was_live,
                                now,
                            );
                            merged
                        })
                        .collect();

                    let new_streams: Vec<NewStream> = merged_chunk
                        .iter()
                        .map(NewStream::from_stream_info)
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

                    if !query.platforms.is_empty() {
                        dq = dq.filter(streams::platform.eq_any(query.platforms.clone()));
                    }

                    if let Some(is_live) = query.is_live {
                        dq = dq.filter(streams::is_live.eq(is_live));
                    }

                    if !query.languages.is_empty() {
                        dq = dq.filter(streams::language.eq_any(query.languages.clone()));
                    }

                    if !query.categories.is_empty() {
                        dq = dq.filter(streams::category.eq_any(query.categories.clone()));
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

                    // Tags column is a JSON string; match if it contains any of the requested
                    // tags. SQLite has no JSON array predicate exposed via diesel here, so we
                    // OR a LIKE per tag. Cheap because the tag list is short and the tag
                    // column already has a string-pattern index pattern via LIKE.
                    if !query.tags.is_empty() {
                        use diesel::BoolExpressionMethods;
                        let mut iter = query.tags.iter();
                        let first = iter.next().unwrap();
                        let mut tag_predicate: Box<
                            dyn diesel::BoxableExpression<
                                streams::table,
                                diesel::sqlite::Sqlite,
                                SqlType = diesel::sql_types::Bool,
                            >,
                        > = Box::new(streams::tags.like(format!("%\"{}%", first)));
                        for t in iter {
                            tag_predicate = Box::new(
                                tag_predicate.or(streams::tags.like(format!("%\"{}%", t))),
                            );
                        }
                        dq = dq.filter(tag_predicate);
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
                ("updated" | "fetched", true) => {
                    data_query.order(streams::last_fetched_at.asc())
                }
                ("updated" | "fetched", false) => {
                    data_query.order(streams::last_fetched_at.desc())
                }
                ("live", true) => data_query.order(streams::last_live_at.asc()),
                ("live", false) => data_query.order(streams::last_live_at.desc()),
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

    // ===== Community Operations =====

    async fn list_communities(&self) -> Result<Vec<Community>, StoreError> {
        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| StoreError::ConnectionError(e.to_string()))?;

            let rows: Vec<CommunityRow> = communities::table
                .select(CommunityRow::as_select())
                .order(communities::updated_at.desc())
                .load(&mut conn)
                .map_err(|e| StoreError::QueryError(e.to_string()))?;

            let mut out = Vec::with_capacity(rows.len());
            for row in rows {
                let mut c = row
                    .to_community()
                    .map_err(|e| StoreError::SerializationError(e.to_string()))?;
                c.domains = community_domains::table
                    .filter(community_domains::slug.eq(&c.slug))
                    .select(community_domains::host)
                    .load::<String>(&mut conn)
                    .map_err(|e| StoreError::QueryError(e.to_string()))?;
                out.push(c);
            }
            Ok(out)
        })
        .await
        .map_err(|e| StoreError::QueryError(format!("Task join error: {}", e)))?
    }

    async fn get_community(&self, slug: &str) -> Result<Option<Community>, StoreError> {
        let slug = slug.to_string();
        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| StoreError::ConnectionError(e.to_string()))?;

            let row: Option<CommunityRow> = communities::table
                .find(&slug)
                .select(CommunityRow::as_select())
                .first(&mut conn)
                .optional()
                .map_err(|e| StoreError::QueryError(e.to_string()))?;

            match row {
                Some(r) => {
                    let mut c = r
                        .to_community()
                        .map_err(|e| StoreError::SerializationError(e.to_string()))?;
                    c.domains = community_domains::table
                        .filter(community_domains::slug.eq(&c.slug))
                        .select(community_domains::host)
                        .load::<String>(&mut conn)
                        .map_err(|e| StoreError::QueryError(e.to_string()))?;
                    Ok(Some(c))
                }
                None => Ok(None),
            }
        })
        .await
        .map_err(|e| StoreError::QueryError(format!("Task join error: {}", e)))?
    }

    async fn get_community_by_domain(
        &self,
        host: &str,
    ) -> Result<Option<Community>, StoreError> {
        let host = host.to_string();
        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| StoreError::ConnectionError(e.to_string()))?;

            let resolved: Option<String> = community_domains::table
                .find(&host)
                .select(community_domains::slug)
                .first(&mut conn)
                .optional()
                .map_err(|e| StoreError::QueryError(e.to_string()))?;

            let Some(slug) = resolved else {
                return Ok(None);
            };

            let row: Option<CommunityRow> = communities::table
                .find(&slug)
                .select(CommunityRow::as_select())
                .first(&mut conn)
                .optional()
                .map_err(|e| StoreError::QueryError(e.to_string()))?;

            match row {
                Some(r) => {
                    let mut c = r
                        .to_community()
                        .map_err(|e| StoreError::SerializationError(e.to_string()))?;
                    c.domains = community_domains::table
                        .filter(community_domains::slug.eq(&c.slug))
                        .select(community_domains::host)
                        .load::<String>(&mut conn)
                        .map_err(|e| StoreError::QueryError(e.to_string()))?;
                    Ok(Some(c))
                }
                None => Ok(None),
            }
        })
        .await
        .map_err(|e| StoreError::QueryError(format!("Task join error: {}", e)))?
    }

    async fn upsert_community(&self, community: &Community) -> Result<Community, StoreError> {
        let incoming = community.clone();
        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| StoreError::ConnectionError(e.to_string()))?;

            // Serialize once outside the transaction (failures here are
            // SerializationError, not DB errors).
            let result: Result<Community, diesel::result::Error> =
                conn.transaction::<_, diesel::result::Error, _>(|conn| {
                    // Resolve created_at: keep prior if exists, otherwise stamp now.
                    let prior: Option<CommunityRow> = communities::table
                        .find(&incoming.slug)
                        .select(CommunityRow::as_select())
                        .first(conn)
                        .optional()?;

                    let now = Utc::now();
                    let mut final_community = incoming.clone();
                    final_community.updated_at = now;
                    final_community.created_at = match &prior {
                        Some(r) => DateTime::parse_from_rfc3339(&r.created_at)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or(now),
                        None => now,
                    };

                    // Reject any incoming domain already claimed by a different slug.
                    if !final_community.domains.is_empty() {
                        let conflicts: Vec<(String, String)> = community_domains::table
                            .filter(community_domains::host.eq_any(&final_community.domains))
                            .filter(community_domains::slug.ne(&final_community.slug))
                            .select((community_domains::host, community_domains::slug))
                            .load(conn)?;
                        if let Some((host, owner)) = conflicts.into_iter().next() {
                            return Err(diesel::result::Error::RollbackTransaction)
                                .map_err(|_| {
                                    diesel::result::Error::QueryBuilderError(
                                        format!(
                                            "domain '{host}' is already claimed by community '{owner}'"
                                        )
                                        .into(),
                                    )
                                });
                        }
                    }

                    let new_row = NewCommunity::from_community(&final_community).map_err(|e| {
                        diesel::result::Error::SerializationError(Box::new(e))
                    })?;

                    diesel::replace_into(communities::table)
                        .values(&new_row)
                        .execute(conn)?;

                    // Atomic replace of the domain set for this slug.
                    diesel::delete(
                        community_domains::table
                            .filter(community_domains::slug.eq(&final_community.slug)),
                    )
                    .execute(conn)?;

                    if !final_community.domains.is_empty() {
                        let created_at = now.to_rfc3339();
                        let new_domain_rows: Vec<NewCommunityDomain> = final_community
                            .domains
                            .iter()
                            .map(|host| NewCommunityDomain {
                                host,
                                slug: &final_community.slug,
                                created_at: created_at.clone(),
                            })
                            .collect();
                        diesel::insert_into(community_domains::table)
                            .values(&new_domain_rows)
                            .execute(conn)?;
                    }

                    Ok(final_community)
                });

            result.map_err(|e| match e {
                diesel::result::Error::QueryBuilderError(msg) => {
                    StoreError::QueryError(msg.to_string())
                }
                diesel::result::Error::SerializationError(e) => {
                    StoreError::SerializationError(e.to_string())
                }
                other => StoreError::QueryError(other.to_string()),
            })
        })
        .await
        .map_err(|e| StoreError::QueryError(format!("Task join error: {}", e)))?
    }

    async fn delete_community(&self, slug: &str) -> Result<bool, StoreError> {
        let slug = slug.to_string();
        let pool = Arc::clone(&self.pool);

        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .map_err(|e| StoreError::ConnectionError(e.to_string()))?;

            // ON DELETE CASCADE on community_domains.slug cleans the join table.
            let n = diesel::delete(communities::table.find(&slug))
                .execute(&mut conn)
                .map_err(|e| StoreError::QueryError(e.to_string()))?;
            Ok(n > 0)
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
    async fn test_diesel_multi_value_filters() {
        let store = DieselStore::memory().unwrap();

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

        let page = store
            .get_streams(&StreamQuery {
                languages: vec!["no".into(), "sv".into()],
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(page.total, 2);

        let page = store
            .get_streams(&StreamQuery {
                platforms: vec!["twitch".into(), "kick".into()],
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(page.total, 2);

        let page = store
            .get_streams(&StreamQuery {
                tags: vec!["nordic".into(), "fps".into()],
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(page.total, 3);
    }

    #[tokio::test]
    async fn test_diesel_community_crud() {
        let store = DieselStore::memory().unwrap();

        let mut c = Community::new("lsn", "LiveStreamNorge", "0.58 0.20 250");
        c.domains = vec!["lsn.local".into(), "livestreamnorge.test".into()];
        c.filter.languages = vec!["no".into()];

        let created = store.upsert_community(&c).await.unwrap();
        assert_eq!(created.slug, "lsn");
        assert_eq!(created.domains.len(), 2);

        let all = store.list_communities().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].domains.len(), 2);

        let got = store.get_community("lsn").await.unwrap().unwrap();
        assert_eq!(got.name, "LiveStreamNorge");
        assert_eq!(got.filter.languages, vec!["no".to_string()]);

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

        // Update: replace the domain set
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

        // Domain conflict: different community can't grab lsn.local
        let mut other = Community::new("sv", "SwedishStreamers", "0.78 0.16 95");
        other.domains = vec!["lsn.local".into()];
        let err = store.upsert_community(&other).await.unwrap_err();
        assert!(format!("{err}").contains("already claimed"));

        // Delete cascades to community_domains via FK
        assert!(store.delete_community("lsn").await.unwrap());
        assert!(store
            .get_community_by_domain("lsn.local")
            .await
            .unwrap()
            .is_none());
        assert!(!store.delete_community("lsn").await.unwrap());
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
