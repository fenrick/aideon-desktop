//! Tauri application setup and lifecycle management.

use std::sync::Mutex;
use tauri::async_runtime::spawn;

use crate::menu::{build_menu, handle_menu_event};
use crate::setup::{SetupState, run_backend_setup};
use crate::windows::create_windows;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .clear_targets()
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Stdout,
                ))
                .level(tauri_plugin_log::log::LevelFilter::Info)
                .build(),
        )
        .manage(Mutex::new(SetupState::new()))
        .setup(|app| {
            build_menu(app)?;

            app.on_menu_event(move |app, event| {
                handle_menu_event(app, event);
            });

            create_windows(app)?;

            let app_handle = app.handle().clone();
            spawn(async move {
                if let Err(error) = run_backend_setup(app_handle).await {
                    log::error!("host: backend setup failed: {error}");
                }
            });

            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            crate::commands::greet,
            crate::setup::set_complete,
            crate::temporal::temporal_state_at,
            crate::temporal::temporal_metamodel_get,
            crate::health::worker_health,
            crate::temporal::temporal_diff,
            crate::temporal::topology_delta,
            crate::temporal::commit_changes,
            crate::temporal::list_commits,
            crate::temporal::create_branch,
            crate::temporal::list_branches,
            crate::temporal::merge_branches,
            crate::scene::canvas_scene,
            crate::scene::canvas_save_layout,
            crate::praxis_api::praxis_graph_view,
            crate::praxis_api::praxis_catalogue_view,
            crate::praxis_api::praxis_matrix_view,
            crate::praxis_api::praxis_apply_operations,
            crate::praxis_api::praxis_list_scenarios,
            crate::mneme::mneme_upsert_metamodel_batch,
            crate::mneme::mneme_compile_effective_schema,
            crate::mneme::mneme_get_effective_schema,
            crate::mneme::mneme_list_edge_type_rules,
            crate::windows::open_about,
            crate::windows::open_settings,
            crate::windows::open_status,
            crate::windows::open_styleguide,
            crate::setup::get_setup_state
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
