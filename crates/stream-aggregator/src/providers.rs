//! Provider registry and initialization

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

use stream_aggregator_core::{HealthStatus, PlatformProvider};

use crate::config::ProvidersConfig;

#[cfg(feature = "provider-twitch")]
use stream_aggregator_provider_twitch::{TwitchConfig, TwitchProvider};

#[cfg(feature = "provider-youtube")]
use stream_aggregator_provider_youtube::{YouTubeConfig, YouTubeProvider};

#[cfg(feature = "provider-kick")]
use stream_aggregator_provider_kick::{KickConfig, KickProvider};

#[cfg(feature = "provider-dlive")]
use stream_aggregator_provider_dlive::{DLiveConfig, DLiveProvider};

#[cfg(feature = "provider-trovo")]
use stream_aggregator_provider_trovo::{TrovoConfig, TrovoProvider};

#[cfg(feature = "provider-guac")]
use stream_aggregator_provider_guac::{GuacConfig, GuacProvider};

#[cfg(feature = "provider-angelthump")]
use stream_aggregator_provider_angelthump::{AngelThumpConfig, AngelThumpProvider};

#[cfg(feature = "provider-robotstreamer")]
use stream_aggregator_provider_robotstreamer::{RobotStreamerConfig, RobotStreamerProvider};

macro_rules! register_providers {
	($registry:expr, $config:expr, [
		$(
			{
				feature: $feature:literal,
				provider: $provider_type:ty,
				config_type: $config_type:ty,
				config_field: $config_field:ident,
				name: $name:literal,
				init: $init_fn:expr
			}
		),* $(,)?
	]) => {
		$(
			#[cfg(feature = $feature)]
			{
				let provider_config: &$config_type = &$config.$config_field;
				if !provider_config.enabled {
					info!(concat!("⏭️  ", $name, " provider disabled in configuration"));
				} else {
					match $init_fn(provider_config).await {
						Ok(provider) => {
							let provider = Arc::new(provider) as Arc<dyn PlatformProvider>;
							match provider.health_check().await {
								HealthStatus::Healthy => {
									info!(concat!("✅ ", $name, " provider initialized and healthy"));
									$registry.providers.push(provider);
								}
								status => {
									anyhow::bail!(concat!($name, " provider health check failed: {:?}"), status);
								}
							}
						}
						Err(e) => {
							warn!(concat!("⚠️  ", $name, " provider disabled: {}"), e);
						}
					}
				}
			}
		)*
	};
}

/// Registry of platform providers
pub struct ProviderRegistry {
    providers: Vec<Arc<dyn PlatformProvider>>,
}

impl ProviderRegistry {
    /// Create an empty registry
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

	/// Register all enabled providers from configuration
	pub async fn register_all(config: &ProvidersConfig) -> Result<Self> {
		let mut registry = Self::new();

		register_providers!(registry, config, [
			{
				feature: "provider-twitch",
				provider: TwitchProvider,
				config_type: crate::config::TwitchProviderConfig,
				config_field: twitch,
				name: "Twitch",
				init: |cfg: &crate::config::TwitchProviderConfig| async {
					let (Some(client_id), Some(client_secret)) = (&cfg.client_id, &cfg.client_secret) else {
						anyhow::bail!("Missing credentials - set TWITCH_CLIENT_ID and TWITCH_CLIENT_SECRET");
					};
					let twitch_config = TwitchConfig::new(client_id.clone(), client_secret.clone());
					Ok(TwitchProvider::new(twitch_config))
				}
			},
			{
				feature: "provider-youtube",
				provider: YouTubeProvider,
				config_type: crate::config::YouTubeProviderConfig,
				config_field: youtube,
				name: "YouTube",
				init: |_cfg: &crate::config::YouTubeProviderConfig| async {
					let youtube_config = YouTubeConfig::default();
					Ok(YouTubeProvider::new(youtube_config))
				}
			},
			{
				feature: "provider-kick",
				provider: KickProvider,
				config_type: crate::config::KickProviderConfig,
				config_field: kick,
				name: "Kick",
				init: |_cfg: &crate::config::KickProviderConfig| async {
					let kick_config = KickConfig::default();
					Ok(KickProvider::new(kick_config))
				}
			},
			{
				feature: "provider-dlive",
				provider: DLiveProvider,
				config_type: crate::config::DLiveProviderConfig,
				config_field: dlive,
				name: "DLive",
				init: |_cfg: &crate::config::DLiveProviderConfig| async {
					let dlive_config = DLiveConfig::default();
					Ok(DLiveProvider::new(dlive_config))
				}
			},
			{
				feature: "provider-trovo",
				provider: TrovoProvider,
				config_type: crate::config::TrovoProviderConfig,
				config_field: trovo,
				name: "Trovo",
				init: |_cfg: &crate::config::TrovoProviderConfig| async {
					let trovo_config = TrovoConfig::default();
					Ok(TrovoProvider::new(trovo_config))
				}
			},
			{
				feature: "provider-guac",
				provider: GuacProvider,
				config_type: crate::config::GuacProviderConfig,
				config_field: guac,
				name: "Guac",
				init: |_cfg: &crate::config::GuacProviderConfig| async {
					let guac_config = GuacConfig::default();
					Ok(GuacProvider::new(guac_config))
				}
			},
			{
				feature: "provider-angelthump",
				provider: AngelThumpProvider,
				config_type: crate::config::AngelThumpProviderConfig,
				config_field: angelthump,
				name: "AngelThump",
				init: |_cfg: &crate::config::AngelThumpProviderConfig| async {
					let angelthump_config = AngelThumpConfig::default();
					Ok(AngelThumpProvider::new(angelthump_config))
				}
			},
			{
				feature: "provider-robotstreamer",
				provider: RobotStreamerProvider,
				config_type: crate::config::RobotStreamerProviderConfig,
				config_field: robotstreamer,
				name: "RobotStreamer",
				init: |_cfg: &crate::config::RobotStreamerProviderConfig| async {
					let robotstreamer_config = RobotStreamerConfig::default();
					Ok(RobotStreamerProvider::new(robotstreamer_config))
				}
			},
		]);

		if registry.providers.is_empty() {
			anyhow::bail!("No providers configured! At least one provider must be enabled.");
		}

		info!("✅ {} provider(s) initialized", registry.providers.len());
		Ok(registry)
	}

    /// Get a provider by platform ID
    pub fn get(&self, platform_id: &str) -> Option<Arc<dyn PlatformProvider>> {
        self.providers
            .iter()
            .find(|p| p.platform_id() == platform_id)
            .cloned()
    }

    /// Get all registered providers
    pub fn list(&self) -> &[Arc<dyn PlatformProvider>] {
        &self.providers
    }

    /// Get count of registered providers
    pub fn len(&self) -> usize {
        self.providers.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    /// Get providers as a HashMap for quick platform_id lookups
    pub fn as_map(&self) -> HashMap<String, Arc<dyn PlatformProvider>> {
        self.providers
            .iter()
            .map(|p| (p.platform_id().to_string(), Arc::clone(p)))
            .collect()
    }

    // ===== Individual Provider Registration Methods =====

    /// Register Twitch provider if configured
    #[cfg(feature = "provider-twitch")]
    async fn register_twitch(&mut self, config: &crate::config::TwitchProviderConfig) -> Result<()> {
        if !config.enabled {
            info!("⏭️  Twitch provider disabled in configuration");
            return Ok(());
        }

        let (Some(client_id), Some(client_secret)) = (&config.client_id, &config.client_secret) else {
            warn!("⚠️  Twitch provider disabled (missing credentials)");
            info!("   Set TWITCH_CLIENT_ID and TWITCH_CLIENT_SECRET to enable");
            return Ok(());
        };

        let twitch_config = TwitchConfig::new(client_id.clone(), client_secret.clone());
        let provider = Arc::new(TwitchProvider::new(twitch_config));

        // Health check
        match provider.health_check().await {
            HealthStatus::Healthy => {
                info!("✅ Twitch provider initialized and healthy");
                self.providers.push(provider);
                Ok(())
            }
            status => {
                anyhow::bail!("Twitch provider health check failed: {:?}", status)
            }
        }
    }

    // Future provider registration methods:
    //
    // #[cfg(feature = "provider-youtube")]
    // async fn register_youtube(&mut self, config: &YoutubeProviderConfig) -> Result<()> {
    //     if !config.enabled { return Ok(()); }
    //     // ... initialize YouTube provider
    //     self.providers.push(provider);
    //     Ok(())
    // }
    //
    // #[cfg(feature = "provider-kick")]
    // async fn register_kick(&mut self, config: &KickProviderConfig) -> Result<()> {
    //     if !config.enabled { return Ok(()); }
    //     // ... initialize Kick provider
    //     self.providers.push(provider);
    //     Ok(())
    // }
    //
    // ... etc for each provider
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
