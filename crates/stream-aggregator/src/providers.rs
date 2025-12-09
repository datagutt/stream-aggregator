//! Provider registry and initialization

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, warn};

use stream_aggregator_core::{HealthStatus, PlatformProvider};

use crate::config::ProvidersConfig;

#[cfg(feature = "provider-twitch")]
use stream_aggregator_provider_twitch::{TwitchConfig, TwitchProvider};

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

        // Register Twitch provider
        #[cfg(feature = "provider-twitch")]
        registry.register_twitch(&config.twitch).await?;

        // Future providers will be registered here:
        // #[cfg(feature = "provider-youtube")]
        // registry.register_youtube(&config.youtube).await?;
        //
        // #[cfg(feature = "provider-kick")]
        // registry.register_kick(&config.kick).await?;
        //
        // ... etc

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
