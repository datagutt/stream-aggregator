use stream_aggregator_core::models::DiscoveryFilters;
use stream_aggregator_core::traits::PlatformProvider;
use stream_aggregator_provider_kick::{KickConfig, KickProvider};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = KickConfig::new();
    let provider = KickProvider::new(config)?;

    println!("Kick discovery support: {}", provider.supports_discovery());

    // Test 1: Basic discovery with English language filter
    println!("\n=== Test 1: English streams, top 5 by viewers ===");
    let filters = DiscoveryFilters {
        languages: vec!["en".to_string()],
        limit: Some(5),
        ..Default::default()
    };

    let discovered = provider.discover_streamers(&filters).await?;
    for streamer in &discovered {
        println!(
            "  {} (@{}) - {} viewers - Category: {} - Language: {}",
            streamer.display_name,
            streamer.user_id,
            streamer.viewer_count.unwrap_or(0),
            streamer.category.as_deref().unwrap_or("Unknown"),
            streamer.language.as_deref().unwrap_or("Unknown")
        );
    }

    // Test 2: Filter by minimum viewers
    println!("\n=== Test 2: English streams with 5,000+ viewers ===");
    let filters = DiscoveryFilters {
        languages: vec!["en".to_string()],
        min_viewers: Some(5000),
        limit: Some(10),
        ..Default::default()
    };

    let discovered = provider.discover_streamers(&filters).await?;
    println!("Found {} streamers with 5,000+ viewers", discovered.len());
    for streamer in &discovered {
        println!(
            "  {} - {} viewers",
            streamer.display_name,
            streamer.viewer_count.unwrap_or(0)
        );
    }

    // Test 3: Filter by category ID (15 = Just Chatting)
    println!("\n=== Test 3: Just Chatting category (category_id=15) ===");
    let filters = DiscoveryFilters {
        categories: vec!["15".to_string()],
        limit: Some(5),
        ..Default::default()
    };

    let discovered = provider.discover_streamers(&filters).await?;
    println!("Found {} Just Chatting streamers", discovered.len());
    for streamer in &discovered {
        println!(
            "  {} - {} viewers - {}",
            streamer.display_name,
            streamer.viewer_count.unwrap_or(0),
            streamer.category.as_deref().unwrap_or("Unknown")
        );
    }

    // Test 4: Multiple languages
    println!("\n=== Test 4: English and Spanish streams ===");
    let filters = DiscoveryFilters {
        languages: vec!["en".to_string(), "es".to_string()],
        limit: Some(5),
        ..Default::default()
    };

    let discovered = provider.discover_streamers(&filters).await?;
    println!("Found {} streamers", discovered.len());
    for streamer in &discovered {
        println!(
            "  {} - Language: {}",
            streamer.display_name,
            streamer.language.as_deref().unwrap_or("Unknown")
        );
    }

    // Test 5: Filter by tag
    println!("\n=== Test 5: Gaming tag ===");
    let filters = DiscoveryFilters {
        tags: vec!["gaming".to_string()],
        languages: vec!["en".to_string()],
        limit: Some(5),
        ..Default::default()
    };

    let discovered = provider.discover_streamers(&filters).await?;
    println!("Found {} streamers with 'gaming' tag", discovered.len());
    for streamer in &discovered {
        println!(
            "  {} - {} viewers - Tags: {:?}",
            streamer.display_name,
            streamer.viewer_count.unwrap_or(0),
            streamer.tags
        );
    }

    // Test 6: Category + Tag combination
    println!("\n=== Test 6: Slots & Casino (28) with 'irl' tag ===");
    let filters = DiscoveryFilters {
        categories: vec!["28".to_string()],
        tags: vec!["irl".to_string()],
        limit: Some(3),
        ..Default::default()
    };

    let discovered = provider.discover_streamers(&filters).await?;
    println!("Found {} Slots streamers with 'irl' tag", discovered.len());
    for streamer in &discovered {
        println!(
            "  {} - {} - {} viewers - Tags: {:?}",
            streamer.display_name,
            streamer.category.as_deref().unwrap_or("Unknown"),
            streamer.viewer_count.unwrap_or(0),
            streamer.tags
        );
    }

    Ok(())
}
