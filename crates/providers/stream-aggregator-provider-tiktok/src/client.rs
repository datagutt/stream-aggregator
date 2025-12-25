//! TikTok provider client implementation using HTTP bridge
//!
//! This implementation spawns and manages an HTTP bridge server that leverages the
//! tiktok-live-connector Node.js library for accessing TikTok Live data.
//!
//! Key features:
//! - Automatic bridge process lifecycle management
//! - HTTP-based communication with the bridge server
//! - Batch request support for efficient multi-user queries
//! - Response caching handled by the bridge

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};
use wreq::Client;

use stream_aggregator_core::{errors::ProviderError, models::*, traits::PlatformProvider};

use crate::models::{
    BatchResultItem, BatchRoomInfoRequest, BatchRoomInfoResponse, HealthResponse, RoomInfoRequest,
    RoomInfoResponse, TikTokConfig, TikTokErrorCode, TikTokStreamData,
};

const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 30;
const DEFAULT_BATCH_SIZE: usize = 20;
const BRIDGE_STARTUP_TIMEOUT_SECS: u64 = 30;
const BRIDGE_HEALTH_CHECK_INTERVAL_MS: u64 = 500;

pub struct TikTokProvider {
    client: Client,
    config: TikTokConfig,
    bridge_process: Arc<Mutex<Option<Child>>>,
}

impl TikTokProvider {
    /// Create a new TikTok provider and start the bridge process
    pub async fn new(config: TikTokConfig) -> Result<Self, anyhow::Error> {
        let timeout_secs = config
            .request_timeout_ms
            .map(|ms| ms / 1000)
            .unwrap_or(DEFAULT_REQUEST_TIMEOUT_SECS);

        let client = Client::builder()
            .user_agent("StreamAggregator/1.0")
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .unwrap_or_else(|_| Client::new());

        let provider = Self {
            client,
            config,
            bridge_process: Arc::new(Mutex::new(None)),
        };

        provider.start_bridge().await?;

        Ok(provider)
    }

    /// Find the bridge directory path
    fn find_bridge_path(&self) -> Result<PathBuf, anyhow::Error> {
        if let Some(ref path) = self.config.bridge_path {
            let path = PathBuf::from(path);
            if path.exists() {
                return Ok(path);
            }
            anyhow::bail!("Configured bridge path does not exist: {}", path.display());
        }

        // Try to find the bridge relative to the executable
        let exe_path = std::env::current_exe()?;
        let exe_dir = exe_path.parent().unwrap_or(&exe_path);

        // Common locations to check
        let search_paths = [
            // Development: relative to workspace root
            exe_dir.join("../../crates/providers/stream-aggregator-provider-tiktok/nodejs-bridge"),
            // Installed: next to executable
            exe_dir.join("nodejs-bridge"),
            exe_dir.join("tiktok-bridge"),
            // Current directory
            PathBuf::from("nodejs-bridge"),
            PathBuf::from("crates/providers/stream-aggregator-provider-tiktok/nodejs-bridge"),
        ];

        for path in &search_paths {
            let canonical = path.canonicalize();
            if let Ok(p) = canonical {
                if p.join("index.js").exists() {
                    return Ok(p);
                }
            }
        }

        anyhow::bail!(
			"Could not find TikTok bridge. Set 'bridge_path' in config or ensure nodejs-bridge directory exists"
		)
    }

    /// Extract port from bridge URL
    fn get_bridge_port(&self) -> u16 {
        self.config
            .bridge_url
            .split(':')
            .next_back()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3456)
    }

    /// Start the Node.js bridge process
    async fn start_bridge(&self) -> Result<(), anyhow::Error> {
        let bridge_path = self.find_bridge_path()?;
        let port = self.get_bridge_port();

        info!(
            "Starting TikTok bridge from {} on port {}",
            bridge_path.display(),
            port
        );

        let child = Command::new("node")
            .arg("index.js")
            .current_dir(&bridge_path)
            .env("TIKTOK_BRIDGE_PORT", port.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn bridge process: {}", e))?;

        {
            let mut process = self.bridge_process.lock().await;
            *process = Some(child);
        }

        self.wait_for_bridge_ready().await?;

        info!("TikTok bridge started successfully");
        Ok(())
    }

    /// Wait for the bridge to become ready
    async fn wait_for_bridge_ready(&self) -> Result<(), anyhow::Error> {
        let health_url = self.bridge_url("/health");
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(BRIDGE_STARTUP_TIMEOUT_SECS);

        loop {
            if start.elapsed() > timeout {
                self.stop_bridge().await;
                anyhow::bail!(
                    "TikTok bridge failed to start within {} seconds",
                    BRIDGE_STARTUP_TIMEOUT_SECS
                );
            }

            // Check if process is still running
            {
                let mut process = self.bridge_process.lock().await;
                if let Some(ref mut child) = *process {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            anyhow::bail!(
                                "Bridge process exited unexpectedly with status: {}",
                                status
                            );
                        }
                        Ok(None) => {}
                        Err(e) => {
                            anyhow::bail!("Failed to check bridge process status: {}", e);
                        }
                    }
                }
            }

            if let Ok(response) = self.client.get(&health_url).send().await {
                if let Ok(health) = response.json::<HealthResponse>().await {
                    if health.status == "ok" {
                        return Ok(());
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(BRIDGE_HEALTH_CHECK_INTERVAL_MS)).await;
        }
    }

    /// Stop the bridge process
    async fn stop_bridge(&self) {
        let mut process = self.bridge_process.lock().await;
        if let Some(mut child) = process.take() {
            debug!("Stopping TikTok bridge process");
            let _ = child.kill().await;
        }
    }

    fn get_batch_size(&self) -> usize {
        self.config.batch_size.unwrap_or(DEFAULT_BATCH_SIZE)
    }

    fn bridge_url(&self, path: &str) -> String {
        format!("{}{}", self.config.bridge_url, path)
    }

    async fn get_room_info(&self, username: &str) -> Result<RoomInfoResponse, ProviderError> {
        let url = self.bridge_url("/room");
        let request = RoomInfoRequest {
            username: username.to_string(),
        };

        debug!("Fetching TikTok room info for user: {}", username);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                ProviderError::HttpError(format!("Failed to connect to TikTok bridge: {}", e))
            })?;

        let status = response.status();
        if status == 404 {
            return Err(ProviderError::HttpError(
                "TikTok bridge endpoint not found. Is the bridge server running?".to_string(),
            ));
        }

        let response_text = response.text().await.map_err(|e| {
            ProviderError::ParseError(format!("Failed to read bridge response: {}", e))
        })?;

        let room_response: RoomInfoResponse =
            serde_json::from_str(&response_text).map_err(|e| {
                error!(
                    "Failed to parse bridge response for {}: {} - body: {}",
                    username, e, response_text
                );
                ProviderError::ParseError(format!("Failed to parse bridge response: {}", e))
            })?;

        if !room_response.success && room_response.data.is_none() {
            let error_code = TikTokErrorCode::from(room_response.error_code.as_deref());
            let error_msg = room_response
                .error
                .unwrap_or_else(|| "Unknown error".to_string());

            warn!(
                "TikTok bridge error for {}: {} (code: {:?})",
                username, error_msg, error_code
            );

            return match error_code {
                TikTokErrorCode::UserNotFound => {
                    Err(ProviderError::StreamerNotFound(username.to_string()))
                }
                TikTokErrorCode::RateLimited => Err(ProviderError::RateLimitExceeded),
                TikTokErrorCode::UserOffline => {
                    // User offline is not an error - return success with is_live = false
                    Ok(RoomInfoResponse {
                        success: true,
                        data: Some(crate::models::TikTokStreamData::offline(username)),
                        error: None,
                        error_code: None,
                        retry_after: None,
                    })
                }
                _ => Err(ProviderError::HttpError(error_msg)),
            };
        }

        Ok(room_response)
    }

    async fn get_room_info_batch(
        &self,
        usernames: Vec<String>,
    ) -> Result<BatchRoomInfoResponse, ProviderError> {
        let url = self.bridge_url("/batch");
        let request = BatchRoomInfoRequest { usernames };

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                ProviderError::HttpError(format!("Failed to connect to TikTok bridge: {}", e))
            })?;

        response.json().await.map_err(|e| {
            ProviderError::ParseError(format!("Failed to parse batch response: {}", e))
        })
    }

    fn room_data_to_stream_info(&self, user_id: &str, data: &TikTokStreamData) -> StreamInfo {
        let mut stream_info = StreamInfo::new("tiktok", user_id, &data.display_name);
        stream_info.is_live = data.live;
        stream_info.avatar_url = data.avatar_url.clone();
        stream_info.thumbnail_url = data.thumbnail_url.clone();
        stream_info.viewer_count = data.viewer_count;
        stream_info.title = data.title.clone();
        stream_info.last_updated = Utc::now();

        stream_info
    }

    fn batch_result_to_stream_info(
        &self,
        result: &BatchResultItem,
    ) -> Result<StreamInfo, ProviderError> {
        if !result.success {
            let error_code = TikTokErrorCode::from(result.error_code.as_deref());
            let error_msg = result
                .error
                .clone()
                .unwrap_or_else(|| "Unknown error".to_string());

            return match error_code {
                TikTokErrorCode::UserNotFound => {
                    Err(ProviderError::StreamerNotFound(result.username.clone()))
                }
                TikTokErrorCode::RateLimited => Err(ProviderError::RateLimitExceeded),
                _ => Err(ProviderError::HttpError(error_msg)),
            };
        }

        let data = result
            .data
            .as_ref()
            .ok_or_else(|| ProviderError::ParseError("No data in batch result".to_string()))?;

        Ok(self.room_data_to_stream_info(&result.username, data))
    }
}

impl Drop for TikTokProvider {
    fn drop(&mut self) {
        // Note: The process has kill_on_drop(true) so it will be killed automatically
        debug!("TikTokProvider dropped, bridge process will be terminated");
    }
}

#[async_trait]
impl PlatformProvider for TikTokProvider {
    fn platform_id(&self) -> &'static str {
        "tiktok"
    }

    fn display_name(&self) -> &'static str {
        "TikTok"
    }

    fn base_url(&self) -> &'static str {
        "https://www.tiktok.com"
    }

    async fn resolve_user_id(&self, username_or_id: &str) -> Result<String, ProviderError> {
        Ok(username_or_id.to_string())
    }

    async fn fetch_stream(&self, user_id: &str) -> Result<StreamInfo, ProviderError> {
        debug!("Fetching TikTok stream for user: {}", user_id);

        let response = self.get_room_info(user_id).await?;

        if !response.success {
            return Err(ProviderError::HttpError(
                response
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        let data = response
            .data
            .ok_or_else(|| ProviderError::ParseError("No data in bridge response".to_string()))?;

        Ok(self.room_data_to_stream_info(user_id, &data))
    }

    async fn fetch_streams_batch(
        &self,
        user_ids: &[String],
    ) -> Vec<Result<StreamInfo, ProviderError>> {
        if user_ids.is_empty() {
            return Vec::new();
        }

        let batch_size = self.get_batch_size();
        let mut all_results = Vec::with_capacity(user_ids.len());

        for chunk in user_ids.chunks(batch_size) {
            let usernames: Vec<String> = chunk.to_vec();
            let chunk_len = usernames.len();

            match self.get_room_info_batch(usernames.clone()).await {
                Ok(batch_response) => {
                    if !batch_response.success {
                        let error = ProviderError::HttpError("Batch request failed".to_string());
                        for _ in 0..chunk_len {
                            all_results.push(Err(error.clone()));
                        }
                        continue;
                    }

                    for result in &batch_response.results {
                        all_results.push(self.batch_result_to_stream_info(result));
                    }
                }
                Err(e) => {
                    warn!(
                        "Batch request failed, falling back to individual requests: {}",
                        e
                    );
                    for username in chunk {
                        all_results.push(self.fetch_stream(username).await);
                    }
                }
            }
        }

        all_results
    }

    fn supports_discovery(&self) -> bool {
        false
    }

    async fn discover_streamers(
        &self,
        _filters: &DiscoveryFilters,
    ) -> Result<Vec<DiscoveredStreamer>, ProviderError> {
        Err(ProviderError::DiscoveryNotSupported)
    }

    fn rate_limit_config(&self) -> RateLimitConfig {
        RateLimitConfig {
            requests_per_minute: self.config.requests_per_minute.unwrap_or(60),
            burst_size: self.config.burst_size.unwrap_or(10),
        }
    }

    async fn health_check(&self) -> HealthStatus {
        let url = self.bridge_url("/health");

        match self.client.get(&url).send().await {
            Ok(response) => match response.json::<HealthResponse>().await {
                Ok(health) if health.status == "ok" => HealthStatus::Healthy,
                Ok(_) => {
                    warn!("TikTok bridge health check returned non-ok status");
                    HealthStatus::Unhealthy
                }
                Err(e) => {
                    warn!("Failed to parse TikTok bridge health response: {}", e);
                    HealthStatus::Unhealthy
                }
            },
            Err(e) => {
                error!("TikTok bridge health check error: {}", e);
                HealthStatus::Unhealthy
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = TikTokConfig::default();
        assert_eq!(config.bridge_url, "http://127.0.0.1:3456");
        assert!(config.bridge_path.is_none());
    }

    #[test]
    fn test_error_code_parsing() {
        assert_eq!(
            TikTokErrorCode::from(Some("user_not_found")),
            TikTokErrorCode::UserNotFound
        );
        assert_eq!(
            TikTokErrorCode::from(Some("rate_limited")),
            TikTokErrorCode::RateLimited
        );
        assert_eq!(
            TikTokErrorCode::from(Some("unknown")),
            TikTokErrorCode::Unknown
        );
        assert_eq!(TikTokErrorCode::from(None), TikTokErrorCode::Unknown);
    }
}
