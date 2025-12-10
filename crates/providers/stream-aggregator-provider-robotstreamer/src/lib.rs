//! # RobotStreamer Platform Provider
//!
//! HTTP-only API for RobotStreamer

mod client;
mod models;

pub use client::RobotStreamerProvider;
pub use models::RobotStreamerConfig;
