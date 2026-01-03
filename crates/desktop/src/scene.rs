//! Host IPC commands for scene/canvas data.

use aideon_chrona::scene::generate_demo_scene;
use aideon_praxis::continuum::{FileSnapshotStore, SnapshotStore};
use aideon_praxis::praxis::canvas::{CanvasLayoutSaveRequest, CanvasShape};
use log::info;

use crate::ipc::HostError;

/// Return a raw scene for the canvas. The renderer performs layout when needed.
#[tauri::command]
pub async fn canvas_scene(as_of: Option<String>) -> Result<Vec<CanvasShape>, HostError> {
    info!("host: canvas_scene requested as_of={:?}", as_of);
    // Return raw scene primitives; renderer performs layout via elkjs by default.
    let shapes = generate_demo_scene();
    info!("host: canvas_scene returning {} shapes (raw)", shapes.len());
    Ok(shapes)
}

/// Resolve the on-disk path used to persist a canvas layout snapshot for a document and asOf.
fn canvas_store_key(doc_id: &str, as_of: &str) -> String {
    format!("canvas/{}/layout-{}.json", doc_id, as_of)
}

/// Persist a canvas layout snapshot (geometry, z-order, grouping) for a document and asOf.
#[tauri::command]
pub async fn canvas_save_layout(payload: CanvasLayoutSaveRequest) -> Result<(), HostError> {
    info!(
        "host: canvas_save_layout doc_id={} as_of={} nodes={} edges={} groups={}",
        payload.doc_id,
        payload.as_of,
        payload.nodes.len(),
        payload.edges.len(),
        payload.groups.len()
    );
    let base = dirs::data_dir()
        .ok_or_else(|| HostError::internal("no data dir"))?
        .join("AideonPraxis");
    let store = FileSnapshotStore::new(base.clone());
    let key = canvas_store_key(&payload.doc_id, &payload.as_of);
    let json = serde_json::to_vec_pretty(&payload)
        .map_err(|e| HostError::internal(format!("serialize failed: {e}")))?;
    store
        .put(&key, &json)
        .map_err(|e| HostError::internal(e.to_string()))?;
    info!("host: canvas_save_layout wrote {}/{}", base.display(), key);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_key_is_stable() {
        let key = canvas_store_key("doc1", "2025-01-01");
        assert_eq!(key, "canvas/doc1/layout-2025-01-01.json");
    }

    #[tokio::test]
    async fn canvas_scene_returns_shapes() {
        let shapes = canvas_scene(None).await.unwrap();
        assert!(!shapes.is_empty());
    }
}
