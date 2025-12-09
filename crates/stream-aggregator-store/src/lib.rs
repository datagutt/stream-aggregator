//! # stream-aggregator-store
//!
//! Storage implementations for StreamAggregator.
//!
//! This crate provides multiple storage backends:
//! - Memory (default) - In-memory with dashmap
//! - SQLite - Persistent, single-file
//! - PostgreSQL - Scalable, multi-instance

#[cfg(feature = "memory")]
pub mod memory;

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "memory")]
pub use memory::MemoryStore;

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteStore;

#[cfg(feature = "postgres")]
pub use postgres::PostgresStore;
