//! Store registry and initialization

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, warn};

use stream_aggregator_core::traits::StreamStore;
use stream_aggregator_store::MemoryStore;

#[cfg(feature = "sqlite-store")]
use stream_aggregator_store::SqliteStore;

use crate::config::StoreConfig;

/// Registry of storage backends
pub struct StoreRegistry {
    store: Arc<dyn StreamStore>,
}

impl StoreRegistry {
    /// Create a new registry with the configured store
    pub async fn register(config: &StoreConfig) -> Result<Self> {
        info!("🗄️  Initializing storage backend: {}", config.backend);

        let store: Arc<dyn StreamStore> = match config.backend.as_str() {
            "memory" => {
                let store = MemoryStore::new();
                info!("✅ Memory store initialized");
                Arc::new(store)
            }

            #[cfg(feature = "sqlite-store")]
            "sqlite" => {
                let database_url = config
                    .database_url
                    .as_deref()
                    .unwrap_or("stream_aggregator.db");

                let store = SqliteStore::new(database_url)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to initialize SQLite store: {}", e))?;
                info!(
                    "✅ SQLite store initialized with database: {}",
                    database_url
                );
                Arc::new(store)
            }

            #[cfg(not(feature = "sqlite-store"))]
            "sqlite" => {
                anyhow::bail!("SQLite store not available - enable 'sqlite-store' feature");
            }

            other => {
                anyhow::bail!("Unknown store backend: {}", other);
            }
        };

        Ok(Self { store })
    }

    /// Get the registered store
    pub fn get(&self) -> Arc<dyn StreamStore> {
        self.store.clone()
    }
}

impl Clone for StoreRegistry {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
        }
    }
}
