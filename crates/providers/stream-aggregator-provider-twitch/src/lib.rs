//! # Twitch Platform Provider
//!
//! Provides integration with Twitch's Helix API, including:
//! - OAuth2 Client Credentials authentication
//! - Stream information fetching (single and batch)
//! - Discovery by categories and tags
//! - Automatic token refresh

mod auth;
mod client;
mod models;

pub use client::TwitchProvider;
pub use models::{TwitchConfig, TwitchError};

#[cfg(test)]
mod tests;
