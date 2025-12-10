//! YouTube-specific data models

/// Configuration for the YouTube provider
#[derive(Debug, Clone)]
pub struct YouTubeConfig {
    // No API key - pure scraping only
}

impl Default for YouTubeConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl YouTubeConfig {
    pub fn new() -> Self {
        Self {}
    }
}
