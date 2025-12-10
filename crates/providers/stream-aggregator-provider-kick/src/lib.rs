//! # Kick Platform Provider
//!
//! Provides integration with Kick.com using wreq for TLS fingerprint emulation:
//! - Browser TLS fingerprint spoofing (required for Cloudflare bypass)
//! - XSRF token handling
//! - Channel API integration

mod client;
mod models;

pub use client::KickProvider;
pub use models::KickConfig;
