//! Example of using SQLite store with StreamAggregator

use stream_aggregator_store::SqliteStore;
use stream_aggregator_core::{StreamInfo, TrackedStreamer, DiscoveryRule};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create SQLite store
    let store = SqliteStore::new("example.db").await?;
    
    // Create a test stream
    let stream = StreamInfo::new("twitch", "test_user", "TestStreamer");
    store.upsert_stream(&stream).await?;
    
    // Create a tracked streamer
    let streamer = TrackedStreamer::new_manual("twitch", "test_user");
    store.add_tracked_streamer(&streamer).await?;
    
    // Create a discovery rule
    let rule = DiscoveryRule::new("test_rule", "Test Rule", "twitch");
    store.add_discovery_rule(&rule).await?;
    
    // Query data back
    let streams = store.get_streams(&Default::default()).await?;
    println!("Found {} streams", streams.items.len());
    
    let tracked_streamers = store.get_tracked_streamers(&Default::default()).await?;
    println!("Tracking {} streamers", tracked_streamers.len());
    
    let rules = store.get_discovery_rules(None).await?;
    println!("Have {} discovery rules", rules.len());
    
    Ok(())
}