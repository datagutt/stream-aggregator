//! # Guac Platform Provider
//!
//! Simple REST API for Guac.live

mod client;
mod models;

pub use client::GuacProvider;
pub use models::GuacConfig;
