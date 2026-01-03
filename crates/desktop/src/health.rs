//! Host-level health commands exposed to the renderer.

use crate::worker::WorkerState;
use aideon_praxis::mneme::WorkerHealth;
use log::{debug, info};
use tauri::State;

#[tauri::command]
/// Return the current worker health snapshot.
pub async fn worker_health(state: State<'_, WorkerState>) -> Result<WorkerHealth, String> {
    info!("host: worker_health requested");
    let snapshot = health_snapshot(state.inner());
    debug!(
        "host: worker_health responding ok={} timestamp={}",
        snapshot.ok, snapshot.timestamp_ms
    );
    Ok(snapshot)
}

fn health_snapshot(state: &WorkerState) -> WorkerHealth {
    state.health()
}

#[cfg(test)]
mod tests {
    use super::health_snapshot;
    use crate::worker::WorkerState;
    use aideon_chrona::TemporalEngine;
    use aideon_praxis::mneme::open_store;
    use tempfile::tempdir;

    #[tokio::test]
    async fn health_snapshot_returns_healthy_state() {
        let dir = tempdir().expect("tempdir");
        let mneme = open_store(dir.path()).await.expect("open store");
        let engine = TemporalEngine::new().await.expect("engine");
        let state = WorkerState::new(engine, mneme);
        let snapshot = health_snapshot(&state);
        assert!(snapshot.ok);
        assert!(snapshot.timestamp_ms > 0);
    }
}
