//! StreamAggregator - Multi-platform live stream aggregation service

use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use tracing::{info, error};
use tracing_subscriber::prelude::*;

use stream_aggregator_api::{create_router_with_auth, AuthConfig};
use stream_aggregator_core::PlatformProvider;
use stream_aggregator_store::MemoryStore;

#[cfg(feature = "provider-twitch")]
use stream_aggregator_provider_twitch::{TwitchProvider, TwitchConfig};

/// StreamAggregator CLI
#[derive(Parser)]
#[command(name = "stream-aggregator")]
#[command(about = "Multi-platform live stream aggregator", version, author)]
struct Cli {
    /// Server host to bind to
    #[arg(long, default_value = "127.0.0.1", env = "HOST")]
    host: String,

    /// Server port to bind to
    #[arg(short, long, default_value = "8080", env = "PORT")]
    port: u16,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info", env = "RUST_LOG")]
    log_level: String,

    /// API keys (comma-separated) for authentication
    #[arg(long, env = "API_KEYS")]
    api_keys: Option<String>,

    /// Require authentication for all requests (including reads)
    #[arg(long, env = "REQUIRE_AUTH_ALL")]
    require_auth_all: bool,

    /// Twitch Client ID
    #[arg(long, env = "TWITCH_CLIENT_ID")]
    twitch_client_id: Option<String>,

    /// Twitch Client Secret
    #[arg(long, env = "TWITCH_CLIENT_SECRET")]
    twitch_client_secret: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_thread_ids(false)
                .compact(),
        )
        .with(tracing_subscriber::EnvFilter::new(&cli.log_level))
        .init();

    info!("🚀 StreamAggregator v{} starting...", env!("CARGO_PKG_VERSION"));

    // Create store
    let store = Arc::new(MemoryStore::new());
    info!("✅ Memory store initialized");

    // Initialize providers
    let mut providers: Vec<Arc<dyn PlatformProvider>> = Vec::new();

    #[cfg(feature = "provider-twitch")]
    {
        if let (Some(client_id), Some(client_secret)) = (cli.twitch_client_id, cli.twitch_client_secret) {
            let config = TwitchConfig::new(client_id, client_secret);
            let twitch = Arc::new(TwitchProvider::new(config));
            
            // Test the connection
            match twitch.health_check().await {
                stream_aggregator_core::HealthStatus::Healthy => {
                    info!("✅ Twitch provider initialized and healthy");
                    providers.push(twitch);
                }
                status => {
                    error!("❌ Twitch provider health check failed: {:?}", status);
                    return Err(anyhow::anyhow!("Twitch provider initialization failed"));
                }
            }
        } else {
            info!("⚠️  Twitch provider disabled (missing credentials)");
            info!("   Set TWITCH_CLIENT_ID and TWITCH_CLIENT_SECRET to enable");
        }
    }

    if providers.is_empty() {
        error!("❌ No providers configured! At least one provider must be enabled.");
        return Err(anyhow::anyhow!("No providers configured"));
    }

    info!("✅ {} provider(s) initialized", providers.len());

    // Configure authentication
    let auth_config = if let Some(keys) = cli.api_keys {
        let api_keys: Vec<String> = keys.split(',').map(|s| s.trim().to_string()).collect();
        info!("🔒 API authentication enabled with {} key(s)", api_keys.len());
        
        let mut config = AuthConfig::new(api_keys);
        if cli.require_auth_all {
            info!("🔒 Requiring authentication for all requests");
            config = config.require_all();
        } else {
            info!("🔓 Public read access enabled (GET /streams, GET /platforms)");
        }
        config
    } else {
        info!("🔓 Public access mode (no authentication required)");
        AuthConfig::default()
    };

    // Create router
    let router = create_router_with_auth(store.clone(), auth_config);

    // Start server
    let addr = format!("{}:{}", cli.host, cli.port);
    info!("🌐 Starting server on http://{}", addr);
    info!("");
    info!("Available endpoints:");
    info!("  • GET  http://{}/health - Health check", addr);
    info!("  • GET  http://{}/api/v1/streams - List all streams", addr);
    info!("  • GET  http://{}/api/v1/streams/:id - Get stream by ID", addr);
    info!("  • GET  http://{}/api/v1/streamers - List tracked streamers", addr);
    info!("  • POST http://{}/api/v1/streamers - Add streamer to track", addr);
    info!("  • GET  http://{}/api/v1/platforms - List platforms", addr);
    info!("");
    info!("📖 Press Ctrl+C to shutdown");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    axum::serve(listener, router)
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

    Ok(())
}
