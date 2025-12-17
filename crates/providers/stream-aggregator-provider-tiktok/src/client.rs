//! TikTok provider client implementation using Node.js bridge

use std::path::PathBuf;
use std::process::Stdio;

use async_trait::async_trait;
use chrono::Utc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use tracing::{debug, error, warn};

use stream_aggregator_core::{
	errors::ProviderError,
	models::*,
	traits::PlatformProvider,
};

use crate::models::{BridgeCommand, BridgeResponse, TikTokConfig};

pub struct TikTokProvider {
	config: TikTokConfig,
	bridge_process: Mutex<Option<BridgeProcess>>,
}

struct BridgeProcess {
	_child: Child,
	stdin: ChildStdin,
	stdout_reader: Lines<BufReader<ChildStdout>>,
}

impl TikTokProvider {
	pub fn new(config: TikTokConfig) -> Self {
		Self {
			config,
			bridge_process: Mutex::new(None),
		}
	}

	fn get_bridge_path(&self) -> PathBuf {
		if let Some(ref path) = self.config.bridge_path {
			PathBuf::from(path)
		} else {
			let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
			path.push("nodejs-bridge");
			path.push("index.js");
			path
		}
	}

	fn get_node_path(&self) -> String {
		self.config
			.node_path
			.clone()
			.unwrap_or_else(|| "node".to_string())
	}

	async fn ensure_bridge(&self) -> Result<(), ProviderError> {
		let mut process = self.bridge_process.lock().await;

		if process.is_some() {
			return Ok(());
		}

		debug!("Starting TikTok Node.js bridge");

		let bridge_path = self.get_bridge_path();
		let node_path = self.get_node_path();

		if !bridge_path.exists() {
			return Err(ProviderError::ConfigError(format!(
				"TikTok bridge not found at {:?}",
				bridge_path
			)));
		}

		let mut child = Command::new(&node_path)
			.arg(&bridge_path)
			.stdin(Stdio::piped())
			.stdout(Stdio::piped())
			.stderr(Stdio::null())
			.spawn()
			.map_err(|e| {
				ProviderError::ConfigError(format!("Failed to start Node.js bridge: {}", e))
			})?;

		let stdin = child.stdin.take().ok_or_else(|| {
			ProviderError::ConfigError("Failed to get bridge stdin".to_string())
		})?;

		let stdout = child.stdout.take().ok_or_else(|| {
			ProviderError::ConfigError("Failed to get bridge stdout".to_string())
		})?;

		let stdout_reader = BufReader::new(stdout).lines();

		*process = Some(BridgeProcess {
			_child: child,
			stdin,
			stdout_reader,
		});

		debug!("TikTok Node.js bridge started successfully");

		Ok(())
	}

	async fn send_and_receive(&self, command: &BridgeCommand) -> Result<BridgeResponse, ProviderError> {
		self.ensure_bridge().await?;

		let mut process = self.bridge_process.lock().await;
		let bridge = process.as_mut().ok_or_else(|| {
			ProviderError::ConfigError("Bridge process not running".to_string())
		})?;

		let command_json = serde_json::to_string(command)
			.map_err(|e| ProviderError::ParseError(format!("Failed to serialize command: {}", e)))?;

		debug!("Sending command to bridge: {}", command_json);

		bridge
			.stdin
			.write_all(command_json.as_bytes())
			.await
			.map_err(|e| ProviderError::HttpError(format!("Failed to write to bridge: {}", e)))?;

		bridge
			.stdin
			.write_all(b"\n")
			.await
			.map_err(|e| ProviderError::HttpError(format!("Failed to write newline: {}", e)))?;

		bridge
			.stdin
			.flush()
			.await
			.map_err(|e| ProviderError::HttpError(format!("Failed to flush stdin: {}", e)))?;

		let line = bridge.stdout_reader.next_line()
			.await
			.map_err(|e| ProviderError::HttpError(format!("Failed to read from bridge: {}", e)))?
			.ok_or_else(|| ProviderError::HttpError("Bridge closed unexpectedly".to_string()))?;

		debug!("Received response from bridge: {}", line.trim());

		let response: BridgeResponse = serde_json::from_str(&line)
			.map_err(|e| ProviderError::ParseError(format!("Failed to parse bridge response: {}", e)))?;

		Ok(response)
	}

	async fn get_room_info(&self, username: &str) -> Result<BridgeResponse, ProviderError> {
		let command = BridgeCommand {
			action: "get_room_info".to_string(),
			username: username.to_string(),
		};

		self.send_and_receive(&command).await
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
				response.error.unwrap_or_else(|| "Unknown error".to_string()),
			));
		}

		let data = response.data.ok_or_else(|| {
			ProviderError::ParseError("No data in bridge response".to_string())
		})?;

		let mut stream_info = StreamInfo::new("tiktok", user_id, &data.name);
		stream_info.is_live = data.live;
		stream_info.avatar_url = if data.avatar.is_empty() {
			None
		} else {
			Some(data.avatar)
		};
		stream_info.thumbnail_url = data.thumbnail_url;
		stream_info.viewer_count = data.viewers;
		stream_info.title = data.title;
		stream_info.last_updated = Utc::now();

		Ok(stream_info)
	}

	async fn fetch_streams_batch(
		&self,
		user_ids: &[String],
	) -> Vec<Result<StreamInfo, ProviderError>> {
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
			requests_per_minute: 30,
			burst_size: 5,
		}
	}

	async fn health_check(&self) -> HealthStatus {
		let command = BridgeCommand {
			action: "ping".to_string(),
			username: String::new(),
		};

		match self.send_and_receive(&command).await {
			Ok(response) if response.success && response.pong == Some(true) => {
				HealthStatus::Healthy
			}
			Ok(_) => {
				warn!("TikTok bridge ping failed");
				HealthStatus::Unhealthy
			}
			Err(e) => {
				error!("TikTok bridge health check error: {}", e);
				HealthStatus::Unhealthy
			}
		}
	}
}

impl Drop for TikTokProvider {
	fn drop(&mut self) {
		debug!("Dropping TikTokProvider, cleaning up bridge process");
	}
}
