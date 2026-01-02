//! Host-side temporal commands bridging renderer IPC calls to the worker engine.
//!
//! These commands remain thin so that all business logic stays within the worker
//! crate, reinforcing the boundary guidance spelled out in `AGENTS.MD`.

use aideon_praxis::praxis::meta::MetaModelDocument;
use aideon_praxis::praxis::temporal::{
    BranchInfo, CommitChangesRequest, CommitChangesResponse, CommitSummary, CreateBranchRequest,
    DiffArgs, DiffSummary, ListBranchesResponse, ListCommitsResponse, MergeRequest, MergeResponse,
    StateAtArgs, StateAtResult, TopologyDeltaArgs, TopologyDeltaResult,
};
use aideon_praxis::praxis::{PraxisError, PraxisErrorCode};
use log::{debug, error, info};
use serde::Serialize;
use std::time::Instant;
use tauri::State;

use crate::worker::WorkerState;

#[tauri::command]
/// Handle a renderer request for `Temporal.StateAt`, delegating to the worker engine.
///
/// The handler logs the request for traceability and forwards the typed DTOs
/// untouched so the transport format stays stable regardless of runtime.
pub async fn temporal_state_at(
    state: State<'_, WorkerState>,
    payload: StateAtArgs,
) -> Result<StateAtResult, HostError> {
    temporal_state_at_inner(state.engine(), payload).await
}

async fn temporal_state_at_inner(
    engine: &aideon_praxis::chrona::TemporalEngine,
    payload: StateAtArgs,
) -> Result<StateAtResult, HostError> {
    info!("host: temporal_state_at received");
    debug!("host: temporal_state_at payload={:?}", payload);

    let started = Instant::now();
    let output = engine.state_at(payload.clone()).await.map_err(host_error)?;
    let elapsed = started.elapsed();
    info!(
        "host: temporal_state_at ok nodes={} edges={} elapsed_ms={}",
        output.nodes,
        output.edges,
        elapsed.as_millis()
    );
    debug!("host: temporal_state_at completed result={:?}", output);
    Ok(output)
}

#[tauri::command]
/// Compute diff summary statistics between two plateaus or timestamps.
pub async fn temporal_diff(
    state: State<'_, WorkerState>,
    payload: DiffArgs,
) -> Result<DiffSummary, HostError> {
    temporal_diff_inner(state.engine(), payload).await
}

async fn temporal_diff_inner(
    engine: &aideon_praxis::chrona::TemporalEngine,
    payload: DiffArgs,
) -> Result<DiffSummary, HostError> {
    info!("host: temporal_diff received");
    debug!(
        "host: temporal_diff params from={:?} to={:?} scope={:?}",
        payload.from, payload.to, payload.scope
    );
    let summary = engine
        .diff_summary(payload.clone())
        .await
        .map_err(host_error)?;
    info!(
        "host: temporal_diff counts node_adds={} node_mods={} node_dels={} edge_adds={} edge_mods={} edge_dels={}",
        summary.node_adds,
        summary.node_mods,
        summary.node_dels,
        summary.edge_adds,
        summary.edge_mods,
        summary.edge_dels
    );
    debug!("host: temporal_diff completed summary={:?}", summary);
    Ok(summary)
}

#[tauri::command]
pub async fn commit_changes(
    state: State<'_, WorkerState>,
    payload: CommitChangesRequest,
) -> Result<CommitChangesResponse, HostError> {
    let id = commit_changes_inner(state.engine(), payload).await?;
    Ok(CommitChangesResponse { id })
}

async fn commit_changes_inner(
    engine: &aideon_praxis::chrona::TemporalEngine,
    payload: CommitChangesRequest,
) -> Result<String, HostError> {
    engine.commit(payload).await.map_err(host_error)
}

#[tauri::command]
pub async fn list_commits(
    state: State<'_, WorkerState>,
    branch: String,
) -> Result<ListCommitsResponse, HostError> {
    let commits = list_commits_inner(state.engine(), branch.clone()).await?;
    debug!(
        "host: list_commits branch={} count={}",
        branch,
        commits.len()
    );
    Ok(ListCommitsResponse { commits })
}

async fn list_commits_inner(
    engine: &aideon_praxis::chrona::TemporalEngine,
    branch: String,
) -> Result<Vec<CommitSummary>, HostError> {
    engine.list_commits(branch).await.map_err(host_error)
}

#[tauri::command]
pub async fn create_branch(
    state: State<'_, WorkerState>,
    payload: CreateBranchRequest,
) -> Result<BranchInfo, HostError> {
    create_branch_inner(state.engine(), payload).await
}

async fn create_branch_inner(
    engine: &aideon_praxis::chrona::TemporalEngine,
    payload: CreateBranchRequest,
) -> Result<BranchInfo, HostError> {
    engine
        .create_branch(payload.name.clone(), payload.from.clone())
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn list_branches(
    state: State<'_, WorkerState>,
) -> Result<ListBranchesResponse, HostError> {
    Ok(list_branches_inner(state.engine()).await)
}

async fn list_branches_inner(
    engine: &aideon_praxis::chrona::TemporalEngine,
) -> ListBranchesResponse {
    engine.list_branches().await
}

#[tauri::command]
pub async fn merge_branches(
    state: State<'_, WorkerState>,
    payload: MergeRequest,
) -> Result<MergeResponse, HostError> {
    merge_branches_inner(state.engine(), payload).await
}

async fn merge_branches_inner(
    engine: &aideon_praxis::chrona::TemporalEngine,
    payload: MergeRequest,
) -> Result<MergeResponse, HostError> {
    engine.merge(payload).await.map_err(host_error)
}

#[tauri::command]
pub async fn topology_delta(
    state: State<'_, WorkerState>,
    payload: TopologyDeltaArgs,
) -> Result<TopologyDeltaResult, HostError> {
    topology_delta_inner(state.engine(), payload).await
}

async fn topology_delta_inner(
    engine: &aideon_praxis::chrona::TemporalEngine,
    payload: TopologyDeltaArgs,
) -> Result<TopologyDeltaResult, HostError> {
    engine.topology_delta(payload).await.map_err(host_error)
}

#[tauri::command]
pub async fn temporal_metamodel_get(
    state: State<'_, WorkerState>,
) -> Result<MetaModelDocument, HostError> {
    Ok(temporal_metamodel_get_inner(state.engine()).await)
}

async fn temporal_metamodel_get_inner(
    engine: &aideon_praxis::chrona::TemporalEngine,
) -> MetaModelDocument {
    engine.meta_model().await
}

#[derive(Debug, Serialize)]
pub struct HostError {
    code: &'static str,
    message: String,
}

pub(crate) fn host_error(error: PraxisError) -> HostError {
    let code = match error.code() {
        PraxisErrorCode::UnknownBranch => "unknown_branch",
        PraxisErrorCode::UnknownCommit => "unknown_commit",
        PraxisErrorCode::ConcurrencyConflict => "concurrency_conflict",
        PraxisErrorCode::ValidationFailed => "validation_failed",
        PraxisErrorCode::IntegrityViolation => "integrity_violation",
        PraxisErrorCode::MergeConflict => "merge_conflict",
    };
    error!("host: praxis error code={} detail={error}", code);
    HostError {
        code,
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aideon_praxis::chrona::TemporalEngine;
    use aideon_praxis::praxis::temporal::{
        ChangeSet, CommitRef, EdgeVersion, NodeVersion, StateAtArgs, TopologyDeltaArgs,
    };
    use serde_json::json;

    #[test]
    fn host_error_maps_codes() {
        let err = PraxisError::ValidationFailed {
            message: "bad".into(),
        };
        let mapped = host_error(err);
        assert_eq!(mapped.code, "validation_failed");
        assert!(mapped.message.contains("bad"));

        let err = PraxisError::IntegrityViolation {
            message: "dup".into(),
        };
        let mapped = host_error(err);
        assert_eq!(mapped.code, "integrity_violation");
    }

    #[test]
    fn host_error_covers_all_codes() {
        let cases = vec![
            (
                PraxisError::UnknownBranch {
                    branch: "main".to_string(),
                },
                "unknown_branch",
            ),
            (
                PraxisError::UnknownCommit {
                    commit: "abc123".to_string(),
                },
                "unknown_commit",
            ),
            (
                PraxisError::ConcurrencyConflict {
                    branch: "dev".to_string(),
                    expected: Some("a1".to_string()),
                    actual: Some("b2".to_string()),
                },
                "concurrency_conflict",
            ),
            (
                PraxisError::MergeConflict {
                    message: "edge".to_string(),
                },
                "merge_conflict",
            ),
        ];

        for (error, code) in cases {
            let mapped = host_error(error);
            assert_eq!(mapped.code, code);
            assert!(
                mapped
                    .message
                    .contains(code.split('_').next().unwrap_or(""))
            );
        }
    }

    #[tokio::test]
    async fn temporal_command_helpers_cover_core_flows() {
        let engine = TemporalEngine::new().await.expect("engine");
        let base = commit_seed(&engine, "base").await;
        let expanded = commit_with_edge(&engine, "expand", &base).await;

        let state = temporal_state_at_inner(
            &engine,
            StateAtArgs {
                as_of: CommitRef::Id(expanded.clone()),
                scenario: Some("main".into()),
                confidence: None,
            },
        )
        .await
        .expect("state");
        assert!(state.nodes > 0);

        let diff = temporal_diff_inner(
            &engine,
            DiffArgs {
                from: CommitRef::Id(base.clone()),
                to: CommitRef::Id(expanded.clone()),
                scope: None,
            },
        )
        .await
        .expect("diff");
        assert!(diff.node_adds >= 1);

        let commits = list_commits_inner(&engine, "main".to_string())
            .await
            .expect("commits");
        assert!(!commits.is_empty());

        let branches = list_branches_inner(&engine).await;
        assert!(!branches.branches.is_empty());

        let delta = topology_delta_inner(
            &engine,
            TopologyDeltaArgs {
                from: CommitRef::Id(base),
                to: CommitRef::Id(expanded),
            },
        )
        .await
        .expect("delta");
        assert!(delta.node_adds >= 1);
    }

    async fn commit_seed(engine: &TemporalEngine, message: &str) -> String {
        engine
            .commit(CommitChangesRequest {
                branch: "main".into(),
                parent: None,
                author: Some("tester".into()),
                time: None,
                message: message.to_string(),
                tags: vec![],
                changes: ChangeSet {
                    node_creates: vec![NodeVersion {
                        id: "cap-1".into(),
                        r#type: Some("Capability".into()),
                        props: Some(json!({ "name": "cap-1" })),
                    }],
                    ..ChangeSet::default()
                },
            })
            .await
            .expect("commit")
    }

    async fn commit_with_edge(engine: &TemporalEngine, message: &str, parent: &str) -> String {
        engine
            .commit(CommitChangesRequest {
                branch: "main".into(),
                parent: Some(parent.to_string()),
                author: None,
                time: None,
                message: message.to_string(),
                tags: vec![],
                changes: {
                    let mut change = ChangeSet::default();
                    change.node_creates.push(NodeVersion {
                        id: "stage-1".into(),
                        r#type: Some("ValueStreamStage".into()),
                        props: Some(json!({ "name": "stage-1" })),
                    });
                    change.edge_creates.push(EdgeVersion {
                        id: None,
                        from: "cap-1".into(),
                        to: "stage-1".into(),
                        r#type: Some("serves".into()),
                        directed: Some(true),
                        props: None,
                    });
                    change
                },
            })
            .await
            .expect("commit")
    }
}
