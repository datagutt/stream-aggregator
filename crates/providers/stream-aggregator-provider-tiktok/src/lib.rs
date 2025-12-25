//! # TikTok Platform Provider
//!
//! Uses Node.js bridge with tiktok-live-connector package.
//!
//! This provider supports batch operations for improved scalability
//! when checking multiple TikTok streams simultaneously.

mod client;
mod models;

pub use client::TikTokProvider;
pub use models::TikTokConfig;
