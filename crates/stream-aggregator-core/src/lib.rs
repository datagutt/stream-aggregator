//! # stream-aggregator-core
//!
//! Core types, traits, and utilities for the StreamAggregator service.
//!
//! This crate provides:
//! - Data models (`StreamInfo`, `TrackedStreamer`, `DiscoveryRule`)
//! - Core traits (`PlatformProvider`, `StreamStore`)
//! - Error types
//! - ID generation utilities

pub mod errors;
pub mod id;
pub mod models;
pub mod traits;

pub use errors::*;
pub use id::*;
pub use models::*;
pub use traits::*;
