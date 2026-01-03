//! Host IPC commands for scene/canvas data.

use aideon_chrona::scene::generate_demo_scene;
use aideon_praxis::continuum::{FileSnapshotStore, SnapshotStore};
use aideon_praxis::praxis::canvas::{CanvasLayoutGetRequest, CanvasLayoutSaveRequest, CanvasShape};
use log::info;
use serde::Deserialize;

use crate::ipc::{HostError, IpcRequest, IpcResponse};

/// Return a raw scene for the canvas. The renderer performs layout when needed.
#[tauri::command]
pub async fn canvas_scene(as_of: Option<String>) -> Result<Vec<CanvasShape>, HostError> {
    info!("host: canvas_scene requested as_of={:?}", as_of);
    // Return raw scene primitives; renderer performs layout via elkjs by default.
    let shapes = generate_demo_scene();
    info!("host: canvas_scene returning {} shapes (raw)", shapes.len());
    Ok(shapes)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasScenePayload {
    #[serde(default)]
    pub as_of: Option<String>,
}

/// Namespaced + requestId-wrapped canvas scene query.
#[tauri::command(rename = "praxis.canvas.get_scene")]
pub async fn praxis_canvas_scene_get(
    request: IpcRequest<CanvasScenePayload>,
) -> Result<IpcResponse<Vec<CanvasShape>>, HostError> {
    let request_id = request.request_id;
    let response = match canvas_scene(request.payload.as_of).await {
        Ok(result) => IpcResponse::ok(request_id, result),
        Err(err) => IpcResponse::err(request_id, err),
    };
    Ok(response)
}

/// Reduce an identifier into a safe path segment (no separators).
fn safe_segment(input: &str) -> String {
    const MAX_LEN: usize = 160;
    let mut out = String::with_capacity(input.len().min(MAX_LEN));
    for ch in input.chars().take(MAX_LEN) {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() { "_".into() } else { out }
}

/// Resolve the on-disk key used to persist a canvas layout snapshot for a document and time context.
fn canvas_store_key(
    doc_id: &str,
    as_of: &str,
    scenario: Option<&str>,
    layer: Option<&str>,
) -> String {
    let doc_id = safe_segment(doc_id);
    let as_of = safe_segment(as_of);
    let scenario = scenario.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(safe_segment(trimmed))
        }
    });
    let layer = layer.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(safe_segment(trimmed))
        }
    });

    let mut path = format!("canvas/{doc_id}");
    if let Some(scenario) = scenario {
        path.push_str(&format!("/scenario-{scenario}"));
    }
    if let Some(layer) = layer {
        path.push_str(&format!("/layer-{layer}"));
    }
    path.push_str(&format!("/layout-{as_of}.json"));
    path
}

fn canvas_snapshot_base() -> Result<std::path::PathBuf, HostError> {
    #[cfg(test)]
    if let Ok(value) = std::env::var("AIDEON_TEST_DATA_DIR") {
        return Ok(std::path::PathBuf::from(value));
    }
    Ok(dirs::data_dir()
        .ok_or_else(|| HostError::internal("no data dir"))?
        .join("AideonPraxis"))
}

fn is_missing_snapshot_error(message: &str) -> bool {
    message.contains("os error 2") || message.contains("No such file")
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
    let base = canvas_snapshot_base()?;
    let store = FileSnapshotStore::new(base.clone());
    let key = canvas_store_key(
        &payload.doc_id,
        &payload.as_of,
        payload.scenario.as_deref(),
        payload.layer.as_deref(),
    );
    let json = serde_json::to_vec_pretty(&payload)
        .map_err(|e| HostError::internal(format!("serialize failed: {e}")))?;
    store
        .put(&key, &json)
        .map_err(|e| HostError::internal(e.to_string()))?;
    info!("host: canvas_save_layout wrote {}/{}", base.display(), key);
    Ok(())
}

/// Load a canvas layout snapshot (if any) for a document and time context.
#[tauri::command]
pub async fn canvas_get_layout(
    payload: CanvasLayoutGetRequest,
) -> Result<Option<CanvasLayoutSaveRequest>, HostError> {
    info!(
        "host: canvas_get_layout doc_id={} as_of={} scenario={:?} layer={:?}",
        payload.doc_id, payload.as_of, payload.scenario, payload.layer
    );
    let base = canvas_snapshot_base()?;
    let store = FileSnapshotStore::new(base.clone());
    let key = canvas_store_key(
        &payload.doc_id,
        &payload.as_of,
        payload.scenario.as_deref(),
        payload.layer.as_deref(),
    );

    match store.get(&key) {
        Ok(bytes) => {
            let layout = serde_json::from_slice::<CanvasLayoutSaveRequest>(&bytes)
                .map_err(|e| HostError::internal(format!("deserialize failed: {e}")))?;
            Ok(Some(layout))
        }
        Err(message) if is_missing_snapshot_error(&message) => Ok(None),
        Err(message) => Err(HostError::internal(message)),
    }
}

/// Namespaced + requestId-wrapped canvas layout load command.
#[tauri::command(rename = "praxis.canvas.get_layout")]
pub async fn praxis_canvas_layout_get(
    request: IpcRequest<CanvasLayoutGetRequest>,
) -> Result<IpcResponse<Option<CanvasLayoutSaveRequest>>, HostError> {
    let request_id = request.request_id;
    let response = match canvas_get_layout(request.payload).await {
        Ok(result) => IpcResponse::ok(request_id, result),
        Err(err) => IpcResponse::err(request_id, err),
    };
    Ok(response)
}

/// Namespaced + requestId-wrapped canvas layout persistence command.
#[tauri::command(rename = "praxis.canvas.save_layout")]
pub async fn praxis_canvas_layout_save(
    request: IpcRequest<CanvasLayoutSaveRequest>,
) -> Result<IpcResponse<()>, HostError> {
    let request_id = request.request_id;
    let response = match canvas_save_layout(request.payload).await {
        Ok(()) => IpcResponse::ok(request_id, ()),
        Err(err) => IpcResponse::err(request_id, err),
    };
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aideon_praxis::praxis::canvas::CanvasNode;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn store_key_is_stable() {
        let key = canvas_store_key("doc1", "2025-01-01", Some("main"), None);
        assert_eq!(key, "canvas/doc1/scenario-main/layout-2025-01-01.json");
    }

    #[tokio::test]
    async fn canvas_scene_returns_shapes() {
        let shapes = canvas_scene(None).await.unwrap();
        assert!(!shapes.is_empty());
    }

    #[tokio::test]
    async fn canvas_layout_roundtrips() {
        let base = std::env::temp_dir().join(format!(
            "aideon-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        unsafe {
            std::env::set_var("AIDEON_TEST_DATA_DIR", base.to_string_lossy().to_string());
        }

        let payload = CanvasLayoutSaveRequest {
            doc_id: "doc-a".into(),
            as_of: "commit-1".into(),
            scenario: Some("main".into()),
            layer: None,
            nodes: vec![CanvasNode {
                id: "w1".into(),
                type_id: "widget".into(),
                x: 10.0,
                y: 20.0,
                w: 100.0,
                h: 50.0,
                z: 0,
                label: None,
                group_id: None,
            }],
            edges: vec![],
            groups: vec![],
        };

        canvas_save_layout(payload.clone()).await.unwrap();

        let loaded = canvas_get_layout(CanvasLayoutGetRequest {
            doc_id: payload.doc_id.clone(),
            as_of: payload.as_of.clone(),
            scenario: payload.scenario.clone(),
            layer: payload.layer.clone(),
        })
        .await
        .unwrap();

        assert_eq!(loaded, Some(payload));

        let _ = fs::remove_dir_all(base);
        unsafe {
            std::env::remove_var("AIDEON_TEST_DATA_DIR");
        }
    }
}
