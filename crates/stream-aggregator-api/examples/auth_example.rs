//! Example showing how to configure API authentication
//!
//! Run with: cargo run --example auth_example

use stream_aggregator_api::{create_router_with_auth, AuthConfig};
use stream_aggregator_store::MemoryStore;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Create a memory store
    let store = Arc::new(MemoryStore::new());

    // Example 1: No authentication (public access)
    println!("=== Example 1: Public Access (No Auth) ===");
    let public_router = create_router_with_auth(store.clone(), AuthConfig::default());
    println!("Router created with public access");
    println!("- GET /api/v1/streams - Public (no auth required)");
    println!("- POST /api/v1/streamers - Public (no auth required)");
    println!();

    // Example 2: API keys required for write operations
    println!("=== Example 2: API Keys for Writes ===");
    let api_keys = vec![
        "secret-key-123".to_string(),
        "another-key-456".to_string(),
    ];
    let write_auth_router = create_router_with_auth(
        store.clone(),
        AuthConfig::new(api_keys.clone()),
    );
    println!("Router created with API key authentication");
    println!("- GET /api/v1/streams - Public (no auth required)");
    println!("- POST /api/v1/streamers - Requires API key");
    println!("- DELETE /api/v1/streamers/... - Requires API key");
    println!();
    println!("Valid API keys:");
    for key in &api_keys {
        println!("  - {}", key);
    }
    println!();
    println!("Usage:");
    println!("  curl -H 'X-API-Key: secret-key-123' -X POST http://localhost:8080/api/v1/streamers");
    println!("  curl 'http://localhost:8080/api/v1/streamers?api_key=secret-key-123'");
    println!();

    // Example 3: API keys required for all operations
    println!("=== Example 3: API Keys for All Operations ===");
    let full_auth_router = create_router_with_auth(
        store.clone(),
        AuthConfig::new(api_keys.clone()).require_all(),
    );
    println!("Router created with strict authentication");
    println!("- GET /api/v1/streams - Requires API key");
    println!("- POST /api/v1/streamers - Requires API key");
    println!("- /health - Still public (health checks always public)");
    println!();

    // Example 4: Using in production
    println!("=== Example 4: Production Setup ===");
    println!("In production, load API keys from environment or config:");
    println!();
    println!("```rust");
    println!("let api_keys = std::env::var(\"API_KEYS\")");
    println!("    .unwrap_or_default()");
    println!("    .split(',')");
    println!("    .map(|s| s.to_string())");
    println!("    .collect::<Vec<_>>();");
    println!();
    println!("let auth_config = if api_keys.is_empty() {{");
    println!("    AuthConfig::default() // No auth");
    println!("}} else {{");
    println!("    AuthConfig::new(api_keys) // Protect writes");
    println!("}};");
    println!();
    println!("let router = create_router_with_auth(store, auth_config);");
    println!("```");
    println!();

    println!("✅ Authentication examples complete!");
}
