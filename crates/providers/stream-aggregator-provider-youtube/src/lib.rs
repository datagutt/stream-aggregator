//! # YouTube Platform Provider
//!
//! Provides integration with YouTube via HTML scraping:
//! - Channel page scraping for metadata
//! - Live page scraping for stream status
//! - No API key required (falls back to scraping)

mod models;
mod scraper;

pub use models::YouTubeConfig;
pub use scraper::YouTubeProvider;
