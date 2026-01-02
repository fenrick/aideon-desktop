//! Worker lifecycle glue for hosting the Chrona temporal engine inside Tauri.
//!
//! The host keeps the engine behind a managed state container so renderer IPC
//! handlers can access it without leaking internal mutability.

use aideon_praxis::chrona::TemporalEngine;
use aideon_praxis::mneme::{MnemeStore, WorkerHealth, open_store};
use aideon_praxis::praxis::PraxisEngine;
use log::{debug, info};
use std::collections::HashMap;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, Wry};
use tokio::sync::{Mutex, oneshot};

/// Shared application state giving command handlers access to the temporal engine.
pub struct WorkerState {
    engine: TemporalEngine,
    mneme: MnemeStore,
    subscriptions: Mutex<HashMap<String, oneshot::Sender<()>>>,
}

impl WorkerState {
    /// Create a new worker state wrapper around the provided engine instance.
    pub fn new(engine: TemporalEngine, mneme: MnemeStore) -> Self {
        debug!("host: WorkerState constructed");
        Self {
            engine,
            mneme,
            subscriptions: Mutex::new(HashMap::new()),
        }
    }

    /// Borrow the underlying temporal engine for read-only operations.
    pub fn engine(&self) -> &TemporalEngine {
        &self.engine
    }

    pub fn mneme(&self) -> &MnemeStore {
        &self.mneme
    }

    pub async fn register_subscription(&self, id: String, cancel: oneshot::Sender<()>) {
        let mut guard = self.subscriptions.lock().await;
        guard.insert(id, cancel);
    }

    pub async fn cancel_subscription(&self, id: &str) -> bool {
        let mut guard = self.subscriptions.lock().await;
        if let Some(cancel) = guard.remove(id) {
            let _ = cancel.send(());
            return true;
        }
        false
    }

    /// Produce a lightweight health snapshot for IPC exposure.
    pub fn health(&self) -> WorkerHealth {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        WorkerHealth::healthy(timestamp_ms)
    }
}

/// Lazily initialize the temporal engine and store it in Tauri managed state.
pub async fn init_temporal(app: &AppHandle<Wry>) -> Result<(), String> {
    let storage_root = app
        .path()
        .app_data_dir()
        .map_err(|err| err.to_string())?
        .join("AideonPraxis")
        .join(".praxis");
    fs::create_dir_all(&storage_root)
        .map_err(|err| format!("failed to prepare storage dir: {err}"))?;
    let db_path = storage_root.join("praxis.sqlite");
    let engine = PraxisEngine::with_sqlite(&db_path)
        .await
        .map_err(|err| format!("temporal engine init failed: {err}"))?;
    let temporal = TemporalEngine::from_engine(engine);
    let mneme_root = storage_root.join("mneme");
    fs::create_dir_all(&mneme_root).map_err(|err| format!("failed to prepare mneme dir: {err}"))?;
    let mneme = open_store(&mneme_root)
        .await
        .map_err(|err| format!("mneme store init failed: {err}"))?;
    app.manage(WorkerState::new(temporal, mneme));
    info!("host: temporal engine registered with application state");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::WorkerState;
    use aideon_praxis::chrona::TemporalEngine;
    use aideon_praxis::mneme::open_store;
    use tempfile::tempdir;
    use tokio::sync::oneshot;

    #[tokio::test]
    async fn worker_state_health_is_ok() {
        let dir = tempdir().expect("tempdir");
        let mneme = open_store(dir.path()).await.expect("open mneme");
        let engine = TemporalEngine::new().await.expect("engine");
        let state = WorkerState::new(engine, mneme);
        let health = state.health();
        assert!(health.ok);
        assert!(health.timestamp_ms > 0);
    }

    #[tokio::test]
    async fn subscriptions_can_register_and_cancel() {
        let dir = tempdir().expect("tempdir");
        let mneme = open_store(dir.path()).await.expect("open mneme");
        let engine = TemporalEngine::new().await.expect("engine");
        let state = WorkerState::new(engine, mneme);
        let (tx, _rx) = oneshot::channel();

        state.register_subscription("sub-1".into(), tx).await;
        assert!(state.cancel_subscription("sub-1").await);
        assert!(!state.cancel_subscription("sub-1").await);
    }
}
