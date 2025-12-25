//! # stream-aggregator-store
//!
//! Storage implementations for StreamAggregator.
//!
//! This crate provides multiple storage backends for maximum deployment flexibility:
//!
//! ## Backends
//!
//! | Backend    | Feature  | Use Case                                    |
//! |------------|----------|---------------------------------------------|
//! | Memory     | `memory` | Testing, development                        |
//! | Diesel     | `diesel` | SQLite with Diesel ORM (Docker, local, VPS) |
//!
//! ## Portability
//!
//! The Diesel backend uses SQLite with proper migrations:
//! - Embedded migrations run automatically on startup
//! - Single-file database for easy deployment
//! - Diesel ORM provides type-safe queries
//!
//! ## Usage
//!
//! ```toml
//! # For production deployment
//! stream-aggregator-store = { version = "0.1", features = ["diesel"] }
//!
//! # For development
//! stream-aggregator-store = { version = "0.1", features = ["memory"] }
//! ```

pub mod schema;

#[cfg(feature = "memory")]
pub mod memory;

#[cfg(feature = "diesel")]
pub mod models;

#[cfg(feature = "diesel")]
pub mod diesel_store;

#[cfg(feature = "memory")]
pub use memory::MemoryStore;

#[cfg(feature = "diesel")]
pub use diesel_store::DieselStore;
