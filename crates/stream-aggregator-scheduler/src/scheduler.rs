//! Scheduler implementation for periodic stream fetching

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use stream_aggregator_core::{PlatformProvider, StreamStore, TrackedStreamerQuery};

/// Configuration for the scheduler
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// How often to scrape each streamer (in seconds)
    pub scrape_interval_secs: u64,

    /// Maximum concurrent scrapes
    pub max_concurrent: usize,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            scrape_interval_secs: 60, // Every minute
            max_concurrent: 10,
        }
    }
}

/// Scheduler for periodic stream information fetching
pub struct Scheduler {
    store: Arc<dyn StreamStore>,
    providers: HashMap<String, Arc<dyn PlatformProvider>>,
    config: SchedulerConfig,
}

impl Scheduler {
    /// Create a new scheduler
    pub fn new(
        store: Arc<dyn StreamStore>,
        providers: Vec<Arc<dyn PlatformProvider>>,
        config: SchedulerConfig,
    ) -> Self {
        let providers_map = providers
            .into_iter()
            .map(|p| (p.platform_id().to_string(), p))
            .collect();

        Self {
            store,
            providers: providers_map,
            config,
        }
    }

    /// Run the scheduler (blocks forever)
    pub async fn run(self) -> ! {
        info!(
            "🕐 Scheduler starting (interval: {}s, max_concurrent: {})",
            self.config.scrape_interval_secs, self.config.max_concurrent
        );

        // Perform initial scrape immediately on startup
        info!("🔄 Performing initial scrape...");
        if let Err(e) = self.scrape_all().await {
            error!("Initial scrape failed: {}", e);
        }

        let mut ticker = interval(Duration::from_secs(self.config.scrape_interval_secs));
        // Skip the first tick since we just did an initial scrape
        ticker.tick().await;

        loop {
            ticker.tick().await;

            if let Err(e) = self.scrape_all().await {
                error!("Scrape cycle failed: {}", e);
            }
        }
    }

    /// Perform one scrape cycle for all tracked streamers
    async fn scrape_all(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Starting scrape cycle");

        // Get all tracked streamers
        let query = TrackedStreamerQuery::default();
        let streamers = self.store.get_tracked_streamers(&query).await?;

        if streamers.is_empty() {
            debug!("No tracked streamers, skipping scrape");
            return Ok(());
        }

        info!("Scraping {} tracked streamer(s)", streamers.len());

        // Create a map of (platform, user_id) -> TrackedStreamer for metadata lookup
        let streamer_metadata: HashMap<(String, String), _> = streamers
            .iter()
            .map(|s| ((s.platform.clone(), s.user_id.clone()), s.clone()))
            .collect();

        // Group streamers by platform for batch fetching
        let mut by_platform: HashMap<String, Vec<String>> = HashMap::new();
        for streamer in streamers {
            by_platform
                .entry(streamer.platform.clone())
                .or_default()
                .push(streamer.user_id.clone());
        }

        // Scrape each platform
        let mut tasks = Vec::new();
        for (platform_id, user_ids) in by_platform {
            let provider = match self.providers.get(&platform_id) {
                Some(p) => p.clone(),
                None => {
                    warn!("No provider for platform '{}', skipping", platform_id);
                    continue;
                }
            };

            let store = self.store.clone();
            let metadata = streamer_metadata.clone();
            let task =
                tokio::spawn(
                    async move { scrape_platform(provider, store, user_ids, metadata).await },
                );
            tasks.push(task);
        }

        // Wait for all platforms to complete
        let results = futures::future::join_all(tasks).await;

        let mut total_success = 0;
        let mut total_failed = 0;

        for result in results {
            match result {
                Ok((success, failed)) => {
                    total_success += success;
                    total_failed += failed;
                }
                Err(e) => {
                    error!("Scrape task panicked: {}", e);
                }
            }
        }

        info!(
            "✅ Scrape cycle complete: {} success, {} failed",
            total_success, total_failed
        );

        Ok(())
    }
}

/// Scrape a single platform's tracked streamers
async fn scrape_platform(
    provider: Arc<dyn PlatformProvider>,
    store: Arc<dyn StreamStore>,
    user_ids: Vec<String>,
    streamer_metadata: HashMap<(String, String), stream_aggregator_core::TrackedStreamer>,
) -> (usize, usize) {
    let platform_id = provider.platform_id();
    debug!(
        "Scraping {} streamer(s) from {}",
        user_ids.len(),
        platform_id
    );

    // Use batch fetching if available
    let results = provider.fetch_streams_batch(&user_ids).await;

    let mut failed_count = 0;
    let mut streams_to_store = Vec::new();

    // Collect all successful stream fetches and enrich them
    for result in results {
        match result {
            Ok(mut stream_info) => {
                // Enrich stream metadata with tracked streamer metadata
                if let Some(tracked) = streamer_metadata
                    .get(&(stream_info.platform.clone(), stream_info.user_id.clone()))
                {
                    // Add group to metadata
                    if let Some(ref group) = tracked.group {
                        stream_info.metadata.insert(
                            "group".to_string(),
                            serde_json::Value::String(group.clone()),
                        );
                    }

                    // Add priority to metadata
                    if let Some(priority) = tracked.priority {
                        stream_info.metadata.insert(
                            "priority".to_string(),
                            serde_json::Value::Number(priority.into()),
                        );
                    }

                    // Add labels to metadata
                    if !tracked.labels.is_empty() {
                        let labels_map: serde_json::Map<String, serde_json::Value> = tracked
                            .labels
                            .iter()
                            .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                            .collect();
                        stream_info
                            .metadata
                            .insert("labels".to_string(), serde_json::Value::Object(labels_map));
                    }
                }

                debug!(
                    "{}: {} is {}",
                    platform_id,
                    stream_info.display_name,
                    if stream_info.is_live {
                        "LIVE"
                    } else {
                        "offline"
                    }
                );

                streams_to_store.push(stream_info);
            }
            Err(e) => {
                warn!("Failed to fetch stream from {}: {}", platform_id, e);
                failed_count += 1;
            }
        }
    }

    // Store all streams in a single batch transaction - much faster than individual upserts!
    let success_count = streams_to_store.len();
    if !streams_to_store.is_empty() {
        match store.batch_upsert_streams(&streams_to_store).await {
            Ok(_) => {
                debug!(
                    "Successfully stored {} streams from {} in batch",
                    success_count, platform_id
                );
            }
            Err(e) => {
                error!(
                    "Failed to batch store {} streams from {}: {}",
                    streams_to_store.len(),
                    platform_id,
                    e
                );
                // All streams in this batch failed
                return (0, streams_to_store.len() + failed_count);
            }
        }
    }

    (success_count, failed_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_config_default() {
        let config = SchedulerConfig::default();
        assert_eq!(config.scrape_interval_secs, 60);
        assert_eq!(config.max_concurrent, 10);
    }
}
