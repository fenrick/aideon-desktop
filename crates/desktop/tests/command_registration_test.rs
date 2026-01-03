const APP_RS: &str = include_str!("../src/app.rs");

#[test]
fn app_registers_core_ipc_commands() {
    let required = [
        "crate::setup::system_setup_complete",
        "crate::windows::system_window_open",
        "crate::health::system_worker_health",
        "crate::workspace::workspace_projects_list",
        "crate::workspace::workspace_templates_list",
        "crate::temporal::praxis_metamodel_get",
        "crate::scene::praxis_canvas_layout_get",
        "crate::scene::praxis_canvas_layout_save",
        "crate::praxis_api::praxis_artefact_graph_execute",
        "crate::praxis_api::praxis_artefact_catalogue_execute",
        "crate::praxis_api::praxis_artefact_matrix_execute",
        "crate::praxis_api::praxis_artefact_chart_execute",
        "crate::praxis_api::praxis_task_apply_operations",
        "crate::praxis_api::praxis_scenario_list",
        "crate::temporal::chrona_temporal_state_at",
        "crate::temporal::chrona_temporal_diff",
        "crate::temporal::chrona_temporal_topology_delta",
        "crate::temporal::chrona_temporal_commit_changes",
        "crate::temporal::chrona_temporal_list_commits",
        "crate::temporal::chrona_temporal_list_branches",
        "crate::temporal::chrona_temporal_create_branch",
        "crate::temporal::chrona_temporal_merge_branches",
    ];

    for path in required {
        assert!(
            APP_RS.contains(path),
            "app.rs missing required invoke handler: {path}"
        );
    }
}

