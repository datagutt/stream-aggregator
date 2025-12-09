//! Configuration management

use serde::{Deserialize, Serialize};

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
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            auth: AuthSettings::default(),
            scheduler: SchedulerConfig::default(),
            providers: ProvidersConfig::default(),
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

/// Provider configurations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProvidersConfig {
    #[serde(default)]
    pub twitch: TwitchProviderConfig,
    
    // Future providers will be added here:
    // pub youtube: YoutubeProviderConfig,
    // pub kick: KickProviderConfig,
    // etc.
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

impl AppConfig {
    /// Load configuration from environment variables and CLI args
    pub fn from_env_and_cli(
        host: String,
        port: u16,
        api_keys: Option<String>,
        require_auth_all: bool,
        scrape_interval_secs: u64,
        twitch_client_id: Option<String>,
        twitch_client_secret: Option<String>,
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
                    enabled: true, // Will be checked in register_twitch
                    client_id: twitch_client_id,
                    client_secret: twitch_client_secret,
                },
            },
        }
    }
}
