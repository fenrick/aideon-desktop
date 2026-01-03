use std::sync::Mutex;

use log::{error, info, warn};
use serde::Deserialize;
use serde::Serialize;
use tauri::{AppHandle, Manager, State, Wry};

use crate::ipc::{EmptyPayload, HostError, IpcRequest, IpcResponse};
use crate::worker::init_temporal;

#[derive(Default)]
pub struct SetupState {
    frontend_task: bool,
    backend_task: bool,
}

impl SetupState {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SetupTask {
    Frontend,
    Backend,
}

fn parse_task(task: &str) -> Option<SetupTask> {
    match task {
        "frontend" => Some(SetupTask::Frontend),
        "backend" => Some(SetupTask::Backend),
        _ => None,
    }
}

fn mark_complete(state: &mut SetupState, task: SetupTask) {
    match task {
        SetupTask::Frontend => state.frontend_task = true,
        SetupTask::Backend => state.backend_task = true,
    }
}

fn all_complete(state: &SetupState) -> bool {
    state.backend_task && state.frontend_task
}

#[derive(Serialize)]
pub struct SetupFlags {
    frontend: bool,
    backend: bool,
}

#[tauri::command]
pub async fn set_complete(
    app: AppHandle<Wry>,
    state: State<'_, Mutex<SetupState>>,
    task: String,
) -> Result<(), HostError> {
    let mut state_lock = state.lock().unwrap();

    let parsed = parse_task(task.as_str()).ok_or_else(|| {
        warn!("host: set_complete called with invalid task '{task}'");
        HostError::invalid_input("invalid task")
    })?;
    info!(
        "host: set_complete({task}) frontend={} backend={}",
        state_lock.frontend_task, state_lock.backend_task
    );
    mark_complete(&mut state_lock, parsed);

    if all_complete(&state_lock) {
        info!("host: setup complete; closing splash and showing main window");
        if let Some(splash) = app.get_webview_window("splash") {
            let _ = splash.close();
        }
        if let Some(main_window) = app.get_webview_window("main") {
            if let Err(error) = main_window.show() {
                warn!("host: failed showing main window: {error}");
            }
            let _ = main_window.set_focus();
        } else {
            warn!("host: main window not found when completing setup");
        }
    }

    Ok(())
}

#[tauri::command]
pub fn get_setup_state(state: State<'_, Mutex<SetupState>>) -> Result<SetupFlags, HostError> {
    let state = state.lock().unwrap();
    Ok(SetupFlags {
        frontend: state.frontend_task,
        backend: state.backend_task,
    })
}

pub async fn run_backend_setup(app: AppHandle<Wry>) -> Result<(), HostError> {
    info!("host: backend setup started");
    init_temporal(&app).await.map_err(HostError::internal)?;

    if let Err(error_message) = set_complete(
        app.clone(),
        app.state::<Mutex<SetupState>>(),
        "backend".to_string(),
    )
    .await
    {
        error!("host: set_complete backend failed: {error_message}");
        return Err(error_message);
    }

    info!("host: backend setup marked complete");
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupCompletePayload {
    pub task: String,
}

/// Namespaced + requestId-wrapped setup completion signal.
#[tauri::command(rename = "system.setup.complete")]
pub async fn system_setup_complete(
    app: AppHandle<Wry>,
    state: State<'_, Mutex<SetupState>>,
    request: IpcRequest<SetupCompletePayload>,
) -> Result<IpcResponse<()>, HostError> {
    let request_id = request.request_id;
    let response = match set_complete(app, state, request.payload.task).await {
        Ok(()) => IpcResponse::ok(request_id, ()),
        Err(err) => IpcResponse::err(request_id, err),
    };
    Ok(response)
}

/// Namespaced + requestId-wrapped setup state query.
#[tauri::command(rename = "system.setup.state")]
pub fn system_setup_state(
    state: State<'_, Mutex<SetupState>>,
    request: IpcRequest<EmptyPayload>,
) -> Result<IpcResponse<SetupFlags>, HostError> {
    let request_id = request.request_id;
    let response = match get_setup_state(state) {
        Ok(flags) => IpcResponse::ok(request_id, flags),
        Err(err) => IpcResponse::err(request_id, err),
    };
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_task_accepts_known_values() {
        assert_eq!(parse_task("frontend"), Some(SetupTask::Frontend));
        assert_eq!(parse_task("backend"), Some(SetupTask::Backend));
        assert_eq!(parse_task("unknown"), None);
    }

    #[test]
    fn marking_tasks_tracks_completion() {
        let mut state = SetupState::new();
        assert!(!all_complete(&state));
        mark_complete(&mut state, SetupTask::Frontend);
        assert!(!all_complete(&state));
        mark_complete(&mut state, SetupTask::Backend);
        assert!(all_complete(&state));
    }
}
