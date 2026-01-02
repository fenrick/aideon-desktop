//! Praxis engine initialization and setup.

use crate::PraxisEngine;
use crate::engine::config::PraxisEngineConfig;
use crate::engine::state::Inner;
use crate::error::PraxisResult;
use crate::store::{MemoryStore, SqliteDb, Store};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

impl PraxisEngine {
    pub async fn new() -> PraxisResult<Self> {
        Self::with_config(PraxisEngineConfig::default()).await
    }

    pub async fn with_config(config: PraxisEngineConfig) -> PraxisResult<Self> {
        Self::with_stores(config, Arc::new(MemoryStore::default())).await
    }

    pub async fn with_sqlite(path: impl AsRef<Path>) -> PraxisResult<Self> {
        Self::with_sqlite_and_config(path, PraxisEngineConfig::default()).await
    }

    pub async fn with_sqlite_and_config(
        path: impl AsRef<Path>,
        config: PraxisEngineConfig,
    ) -> PraxisResult<Self> {
        Self::with_sqlite_inner(path, config, true).await
    }

    pub async fn with_sqlite_unseeded(
        path: impl AsRef<Path>,
        config: PraxisEngineConfig,
    ) -> PraxisResult<Self> {
        Self::with_sqlite_inner(path, config, false).await
    }

    pub async fn with_stores(
        config: PraxisEngineConfig,
        store: Arc<dyn Store>,
    ) -> PraxisResult<Self> {
        Self::with_stores_inner(config, store, true).await
    }

    pub async fn with_stores_unseeded(
        config: PraxisEngineConfig,
        store: Arc<dyn Store>,
    ) -> PraxisResult<Self> {
        Self::with_stores_inner(config, store, false).await
    }

    async fn with_sqlite_inner(
        path: impl AsRef<Path>,
        config: PraxisEngineConfig,
        seed: bool,
    ) -> PraxisResult<Self> {
        let storage = SqliteDb::open(path).await?;
        let commit_store: Arc<dyn Store> = Arc::new(storage.clone());
        Self::with_stores_inner(config, commit_store, seed).await
    }

    async fn with_stores_inner(
        config: PraxisEngineConfig,
        store: Arc<dyn Store>,
        seed: bool,
    ) -> PraxisResult<Self> {
        let inner = Inner::new(config, store).await?;
        let engine = Self {
            inner: Arc::new(Mutex::new(inner)),
        };
        if seed {
            engine.ensure_seeded().await?;
        }
        Ok(engine)
    }
}

impl Default for PraxisEngine {
    fn default() -> Self {
        futures::executor::block_on(Self::new()).expect("in-memory praxis engine init")
    }
}
