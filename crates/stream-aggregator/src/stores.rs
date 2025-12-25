//! Store registry and initialization

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, warn};

use stream_aggregator_core::traits::StreamStore;
use stream_aggregator_store::MemoryStore;

#[cfg(feature = "diesel-store")]
use stream_aggregator_store::DieselStore;

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

            #[cfg(feature = "diesel-store")]
            "diesel" | "sqlite" => {
                let database_url = config
                    .database_url
                    .as_deref()
                    .unwrap_or("stream_aggregator.db");

                let store = DieselStore::new(database_url)
                    .map_err(|e| anyhow::anyhow!("Failed to initialize Diesel store: {}", e))?;
                info!(
                    "✅ Diesel store initialized with database: {}",
                    database_url
                );
                Arc::new(store)
            }

            #[cfg(not(feature = "diesel-store"))]
            "diesel" | "sqlite" => {
                anyhow::bail!("Diesel store not available - enable 'diesel-store' feature");
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
