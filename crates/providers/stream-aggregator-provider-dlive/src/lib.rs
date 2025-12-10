//! # DLive Platform Provider
//!
//! Provides integration with DLive using GraphQL:
//! - GraphQL API for stream data
//! - No authentication required

mod client;
mod models;

pub use client::DLiveProvider;
pub use models::DLiveConfig;
