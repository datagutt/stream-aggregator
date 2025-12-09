//! StreamAggregator - Library exports for the main binary

pub mod config;
pub mod providers;
pub mod stores;

pub use config::AppConfig;
pub use providers::ProviderRegistry;
pub use stores::StoreRegistry;
