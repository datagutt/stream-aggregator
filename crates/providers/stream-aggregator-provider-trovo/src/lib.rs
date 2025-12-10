//! # Trovo Platform Provider
//!
//! Provides integration with Trovo.live using REST API:
//! - Requires Client-ID header
//! - Simple REST API for user and stream data

mod client;
mod models;

pub use client::TrovoProvider;
pub use models::TrovoConfig;
