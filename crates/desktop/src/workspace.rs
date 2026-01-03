//! Minimal workspace navigation commands used by the renderer shell.
//!
//! M0 only requires that the host exposes stable, typed IPC surfaces and that the
//! renderer can discover basic navigation scaffolding without hardcoding backend paths.

use serde::Serialize;

use crate::ipc::{EmptyPayload, HostError, IpcRequest, IpcResponse};
use crate::praxis_api::ScenarioSummary;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPayload {
    pub id: String,
    pub name: String,
    pub scenarios: Vec<ScenarioSummary>,
}

/// List projects for the active workspace.
///
/// M0 scaffolding: returns a single default project, with scenarios derived from the demo
/// scenario source.
#[tauri::command]
pub async fn list_projects() -> Result<Vec<ProjectPayload>, HostError> {
    Ok(vec![ProjectPayload {
        id: "default-project".into(),
        name: "Praxis Workspace".into(),
        scenarios: ScenarioSummary::demo_list(),
    }])
}

/// Namespaced + requestId-wrapped project list query.
#[tauri::command(rename = "workspace.projects.list")]
pub async fn workspace_projects_list(
    request: IpcRequest<EmptyPayload>,
) -> Result<IpcResponse<Vec<ProjectPayload>>, HostError> {
    let request_id = request.request_id;
    let response = match list_projects().await {
        Ok(result) => IpcResponse::ok(request_id, result),
        Err(err) => IpcResponse::err(request_id, err),
    };
    Ok(response)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplatePayload {
    pub id: String,
    pub name: String,
}

/// List canvas templates stored/managed by the host.
///
/// M0 scaffolding: return an empty list and allow the renderer to fall back to built-in
/// templates for preview/dev workflows.
#[tauri::command]
pub async fn list_templates() -> Result<Vec<TemplatePayload>, HostError> {
    Ok(vec![])
}

/// Namespaced + requestId-wrapped template list query.
#[tauri::command(rename = "workspace.templates.list")]
pub async fn workspace_templates_list(
    request: IpcRequest<EmptyPayload>,
) -> Result<IpcResponse<Vec<TemplatePayload>>, HostError> {
    let request_id = request.request_id;
    let response = match list_templates().await {
        Ok(result) => IpcResponse::ok(request_id, result),
        Err(err) => IpcResponse::err(request_id, err),
    };
    Ok(response)
}
