//! Configuration management

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub server: ServerConfig,

    #[serde(default)]
    pub auth: AuthSettings,

    #[serde(default)]
    pub scheduler: SchedulerConfig,

    #[serde(default)]
    pub providers: ProvidersConfig,

    #[serde(default)]
    pub store: StoreConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            auth: AuthSettings::default(),
            scheduler: SchedulerConfig::default(),
            providers: ProvidersConfig::default(),
            store: StoreConfig::default(),
        }
    }
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
        }
    }
}

/// Authentication settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSettings {
    /// API keys for authentication (comma-separated in env var)
    pub api_keys: Vec<String>,

    /// Require authentication for all requests (including reads)
    pub require_all: bool,
}

impl Default for AuthSettings {
    fn default() -> Self {
        Self {
            api_keys: Vec::new(),
            require_all: false,
        }
    }
}

/// Scheduler configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// Scrape interval in seconds
    pub interval_secs: u64,

    /// Maximum concurrent scrape tasks
    pub max_concurrent: usize,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            interval_secs: 300, // 5 minutes
            max_concurrent: 10,
        }
    }
}

/// Store configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreConfig {
    /// Storage backend type ("memory", "sqlite", "postgres")
    pub backend: String,

    /// Database connection URL (for sqlite/postgres)
    pub database_url: Option<String>,
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            backend: "memory".to_string(),
            database_url: None,
        }
    }
}

/// Provider configurations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProvidersConfig {
    #[serde(default)]
    pub twitch: TwitchProviderConfig,

    #[serde(default)]
    pub youtube: YouTubeProviderConfig,

    #[serde(default)]
    pub kick: KickProviderConfig,

    #[serde(default)]
    pub dlive: DLiveProviderConfig,

    #[serde(default)]
    pub trovo: TrovoProviderConfig,

    #[serde(default)]
    pub guac: GuacProviderConfig,

    #[serde(default)]
    pub angelthump: AngelThumpProviderConfig,

    #[serde(default)]
    pub robotstreamer: RobotStreamerProviderConfig,

    #[serde(default)]
    pub tiktok: TikTokProviderConfig,
}

/// Twitch provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitchProviderConfig {
    pub enabled: bool,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
}

impl Default for TwitchProviderConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            client_id: None,
            client_secret: None,
        }
    }
}

/// YouTube provider configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct YouTubeProviderConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Kick provider configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KickProviderConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// DLive provider configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DLiveProviderConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Trovo provider configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrovoProviderConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Guac provider configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GuacProviderConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// AngelThump provider configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AngelThumpProviderConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// RobotStreamer provider configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RobotStreamerProviderConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// TikTok provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TikTokProviderConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// URL of the TikTok bridge HTTP server (default: http://127.0.0.1:3456)
    pub bridge_url: Option<String>,
    /// Path to the Node.js bridge directory (default: auto-detected)
    pub bridge_path: Option<String>,
}

impl Default for TikTokProviderConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            bridge_url: None,
            bridge_path: None,
        }
    }
}

fn default_true() -> bool {
    true
}

impl AppConfig {
    /// Load configuration from TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)?;
        let config: AppConfig = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Load configuration from environment variables and CLI args
    pub fn from_env_and_cli(
        host: String,
        port: u16,
        api_keys: Option<String>,
        require_auth_all: bool,
        scrape_interval_secs: u64,
        twitch_client_id: Option<String>,
        twitch_client_secret: Option<String>,
        store_backend: String,
        database_url: Option<String>,
    ) -> Self {
        let api_keys = api_keys
            .map(|keys| keys.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();

        Self {
            server: ServerConfig { host, port },
            auth: AuthSettings {
                api_keys,
                require_all: require_auth_all,
            },
            scheduler: SchedulerConfig {
                interval_secs: scrape_interval_secs,
                max_concurrent: 10,
            },
            providers: ProvidersConfig {
                twitch: TwitchProviderConfig {
                    enabled: true,
                    client_id: twitch_client_id,
                    client_secret: twitch_client_secret,
                },
                youtube: YouTubeProviderConfig::default(),
                kick: KickProviderConfig::default(),
                dlive: DLiveProviderConfig::default(),
                trovo: TrovoProviderConfig::default(),
                guac: GuacProviderConfig::default(),
                angelthump: AngelThumpProviderConfig::default(),
                robotstreamer: RobotStreamerProviderConfig::default(),
                tiktok: TikTokProviderConfig::default(),
            },
            store: StoreConfig {
                backend: store_backend,
                database_url,
            },
        }
    }
}
