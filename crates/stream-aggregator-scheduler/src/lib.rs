//! # stream-aggregator-scheduler
//!
//! Periodic scraping scheduler for StreamAggregator.
//!
//! This crate provides automatic periodic fetching of stream information
//! for tracked streamers.

mod scheduler;

pub use scheduler::{Scheduler, SchedulerConfig};
