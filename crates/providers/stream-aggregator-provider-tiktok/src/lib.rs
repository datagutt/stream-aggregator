//! # TikTok Platform Provider
//!
//! Uses Node.js bridge with tiktok-live-connector package

mod client;
mod models;

pub use client::TikTokProvider;
pub use models::TikTokConfig;
