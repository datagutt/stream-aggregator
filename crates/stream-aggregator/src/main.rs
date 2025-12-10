//! StreamAggregator - Multi-platform live stream aggregation service

use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::prelude::*;

use stream_aggregator::{AppConfig, ProviderRegistry, StoreRegistry};
use stream_aggregator_api::{create_router_with_auth, AuthConfig};
use stream_aggregator_scheduler::{Scheduler, SchedulerConfig};

/// StreamAggregator CLI
#[derive(Parser)]
#[command(name = "stream-aggregator")]
#[command(about = "Multi-platform live stream aggregator", version, author)]
struct Cli {
    /// Configuration file path (TOML format)
    #[arg(short, long)]
    config: Option<String>,

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

    /// Scrape interval in seconds
    #[arg(long, default_value = "300", env = "SCRAPE_INTERVAL_SECS")]
    scrape_interval_secs: u64,

    /// Twitch Client ID
    #[arg(long, env = "TWITCH_CLIENT_ID")]
    twitch_client_id: Option<String>,

    /// Twitch Client Secret
    #[arg(long, env = "TWITCH_CLIENT_SECRET")]
    twitch_client_secret: Option<String>,

    /// Storage backend type (memory, sqlite)
    #[arg(long, env = "STORE_BACKEND")]
    store_backend: Option<String>,

    /// Database URL (for sqlite/postgres)
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,
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

    // Load configuration
    let config = if let Some(config_path) = &cli.config {
        info!("📄 Loading configuration from file: {}", config_path);
        let mut config = AppConfig::from_file(config_path)
            .map_err(|e| anyhow::anyhow!("Failed to load config from {}: {}", config_path, e))?;

        // Override with CLI args (CLI takes precedence)
        config.server.host = cli.host;
        config.server.port = cli.port;
        config.auth.api_keys = cli.api_keys
            .map(|keys| keys.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or(config.auth.api_keys);
        config.auth.require_all = cli.require_auth_all;
        config.scheduler.interval_secs = cli.scrape_interval_secs;
        if cli.twitch_client_id.is_some() {
            config.providers.twitch.client_id = cli.twitch_client_id;
        }
        if cli.twitch_client_secret.is_some() {
            config.providers.twitch.client_secret = cli.twitch_client_secret;
        }
        if let Some(backend) = cli.store_backend {
            config.store.backend = backend;
        }
        if cli.database_url.is_some() {
            config.store.database_url = cli.database_url;
        }

        config
    } else {
        AppConfig::from_env_and_cli(
            cli.host,
            cli.port,
            cli.api_keys,
            cli.require_auth_all,
            cli.scrape_interval_secs,
            cli.twitch_client_id,
            cli.twitch_client_secret,
            cli.store_backend.unwrap_or("memory".to_string()),
            cli.database_url,
        )
    };

    // Create store
    let store_registry = StoreRegistry::register(&config.store).await?;
    let store = store_registry.get();

    // Register all providers
    let registry = ProviderRegistry::register_all(&config.providers).await?;

    // Start scheduler in background
    let scheduler_store = store.clone();
    let providers: Vec<_> = registry.list().iter().cloned().collect();
    
    let scheduler_config = SchedulerConfig {
        scrape_interval_secs: config.scheduler.interval_secs,
        max_concurrent: config.scheduler.max_concurrent,
    };
    
    let scheduler = Scheduler::new(
        scheduler_store,
        providers,
        scheduler_config,
    );
    
    tokio::spawn(async move {
        scheduler.run().await;
    });

    // Configure authentication
    let auth_config = if !config.auth.api_keys.is_empty() {
        info!("🔒 API authentication enabled with {} key(s)", config.auth.api_keys.len());
        
        let mut auth = AuthConfig::new(config.auth.api_keys);
        if config.auth.require_all {
            info!("🔒 Requiring authentication for all requests");
            auth = auth.require_all();
        } else {
            info!("🔓 Public read access enabled (GET /streams, GET /platforms)");
        }
        auth
    } else {
        info!("🔓 Public access mode (no authentication required)");
        AuthConfig::default()
    };

    // Create router with providers for username resolution
    let provider_map = registry.as_map();
    let router = create_router_with_auth(store, provider_map, auth_config);

    // Start server
    let addr = format!("{}:{}", config.server.host, config.server.port);
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
