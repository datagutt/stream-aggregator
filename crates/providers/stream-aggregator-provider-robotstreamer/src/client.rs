//! RobotStreamer provider client implementation

use async_trait::async_trait;
use chrono::Utc;
use tracing::debug;
use wreq::Client;

use stream_aggregator_core::{
    errors::ProviderError,
    models::*,
    traits::PlatformProvider,
};

use crate::models::{RobotStreamerConfig, RobotStreamerRobot};

/// RobotStreamer platform provider (HTTP only)
pub struct RobotStreamerProvider {
    client: Client,
}

impl RobotStreamerProvider {
    /// Create a new RobotStreamer provider
    pub fn new(_config: RobotStreamerConfig) -> Self {
        let client = Client::builder()
            .user_agent("StreamAggregator/1.0")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client }
    }

    /// Fetch robot info from RobotStreamer API (returns array)
    async fn fetch_robot(&self, robot_id: &str) -> Result<RobotStreamerRobot, ProviderError> {
        debug!("Fetching RobotStreamer robot: {}", robot_id);

        // RobotStreamer uses HTTP (not HTTPS) and specific port 8080
        let url = format!("http://api.robotstreamer.com:8080/v1/get_robot/{}", robot_id);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| ProviderError::HttpError(format!("Failed to fetch RobotStreamer robot: {}", e)))?;

        let status = response.status();
        if status == 404 {
            return Err(ProviderError::StreamerNotFound(robot_id.to_string()));
        }

        if !status.is_success() {
            return Err(ProviderError::HttpError(format!("RobotStreamer API error {}", status)));
        }

        // API returns an array, take first element
        let robots: Vec<RobotStreamerRobot> = response
            .json()
            .await
            .map_err(|e| ProviderError::ParseError(format!("Failed to parse RobotStreamer response: {}", e)))?;

        robots.into_iter().next()
            .ok_or_else(|| ProviderError::StreamerNotFound(robot_id.to_string()))
    }
}

#[async_trait]
impl PlatformProvider for RobotStreamerProvider {
    fn platform_id(&self) -> &'static str {
        "robotstreamer"
    }

    fn display_name(&self) -> &'static str {
        "RobotStreamer"
    }

    fn base_url(&self) -> &'static str {
        "http://robotstreamer.com"
    }

    async fn resolve_user_id(&self, username_or_id: &str) -> Result<String, ProviderError> {
        Ok(username_or_id.to_string())
    }

    async fn fetch_stream(&self, user_id: &str) -> Result<StreamInfo, ProviderError> {
        let robot = self.fetch_robot(user_id).await?;

        // Status indicates if robot is live (e.g., "live" or "offline")
        let is_live = robot.status.as_ref()
            .map(|s| s.to_lowercase() == "live" || s == "1")
            .unwrap_or(false);

        let display_name = robot.robot_name.as_deref().unwrap_or(user_id);

        let mut stream_info = StreamInfo::new("robotstreamer", user_id, display_name);
        stream_info.is_live = is_live;
        stream_info.viewer_count = robot.viewers;
        stream_info.last_updated = Utc::now();

        Ok(stream_info)
    }

    async fn fetch_streams_batch(&self, user_ids: &[String]) -> Vec<Result<StreamInfo, ProviderError>> {
        let mut results = Vec::with_capacity(user_ids.len());
        
        for user_id in user_ids {
            results.push(self.fetch_stream(user_id).await);
        }
        
        results
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
            requests_per_minute: 60,
            burst_size: 10,
        }
    }

    async fn health_check(&self) -> HealthStatus {
        match self.client.get("http://robotstreamer.com").send().await {
            Ok(response) if response.status().is_success() => HealthStatus::Healthy,
            _ => HealthStatus::Unhealthy,
        }
    }
}
