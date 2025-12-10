//! RobotStreamer-specific data models

use serde::Deserialize;

/// Configuration for the RobotStreamer provider
#[derive(Debug, Clone)]
pub struct RobotStreamerConfig {}

impl Default for RobotStreamerConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl RobotStreamerConfig {
    pub fn new() -> Self {
        Self {}
    }
}

/// RobotStreamer robot response (API returns an array)
#[derive(Debug, Deserialize)]
pub struct RobotStreamerRobot {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub robot_name: Option<String>,
    #[serde(default)]
    pub viewers: Option<u64>,
}
