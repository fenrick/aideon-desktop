//! Temporal engine façade built on top of the Praxis commit model.
//!
//! Chrona keeps the IPC-friendly API exposed to the Tauri host while delegating
//! persistence, validation, and diff computation to the Praxis engine.

use aideon_praxis::meta::MetaModelDocument;
use aideon_praxis::temporal::{
    BranchInfo, CommitChangesRequest, CommitRef, CommitSummary, DiffArgs, DiffSummary,
    ListBranchesResponse, MergeRequest, MergeResponse, StateAtArgs, StateAtResult,
    TopologyDeltaArgs, TopologyDeltaResult,
};
use aideon_praxis::{PraxisEngine, PraxisResult};

/// Thin wrapper that keeps the previous `TemporalEngine` name stable for the host.
#[derive(Clone)]
pub struct TemporalEngine {
    inner: PraxisEngine,
}

impl TemporalEngine {
    /// Construct a new temporal engine backed by the default in-memory Praxis store.
    pub async fn new() -> PraxisResult<Self> {
        Ok(Self {
            inner: PraxisEngine::new().await?,
        })
    }

    /// Wrap an existing Praxis engine.
    pub fn from_engine(engine: PraxisEngine) -> Self {
        Self { inner: engine }
    }

    /// Fetch state-at snapshot statistics for the given reference.
    pub async fn state_at(&self, args: StateAtArgs) -> PraxisResult<StateAtResult> {
        self.inner.state_at(args).await
    }

    /// Commit a new change set to the underlying Praxis engine.
    pub async fn commit(&self, request: CommitChangesRequest) -> PraxisResult<String> {
        self.inner.commit(request).await
    }

    /// Create a new branch from the optional reference point.
    pub async fn create_branch(
        &self,
        name: String,
        from: Option<CommitRef>,
    ) -> PraxisResult<BranchInfo> {
        self.inner.create_branch(name, from).await
    }

    /// List commits for the specified branch (oldest to newest).
    pub async fn list_commits(&self, branch: String) -> PraxisResult<Vec<CommitSummary>> {
        self.inner.list_commits(branch).await
    }

    /// Enumerate branches along with their current heads.
    pub async fn list_branches(&self) -> ListBranchesResponse {
        let branches = self.inner.list_branches().await;
        ListBranchesResponse { branches }
    }

    /// Produce a diff summary between two commit references.
    pub async fn diff_summary(&self, args: DiffArgs) -> PraxisResult<DiffSummary> {
        self.inner.diff_summary(args).await
    }

    /// Merge the source branch into the target branch.
    pub async fn merge(&self, request: MergeRequest) -> PraxisResult<MergeResponse> {
        self.inner.merge(request).await
    }

    /// Compute topology deltas between two commit references.
    pub async fn topology_delta(
        &self,
        args: TopologyDeltaArgs,
    ) -> PraxisResult<TopologyDeltaResult> {
        self.inner.topology_delta(args).await
    }

    /// Return the active meta-model document.
    pub async fn meta_model(&self) -> MetaModelDocument {
        self.inner.meta_model().await
    }
}

#[cfg(test)]
mod tests {
    use super::TemporalEngine;
    use aideon_praxis::temporal::{
        ChangeSet, CommitChangesRequest, CommitRef, EdgeTombstone, EdgeVersion, NodeTombstone,
        NodeVersion, StateAtArgs, TopologyDeltaArgs,
    };
    use serde_json::json;

    fn capability_node(id: &str) -> NodeVersion {
        NodeVersion {
            id: id.into(),
            r#type: Some("Capability".into()),
            props: Some(json!({ "name": id })),
        }
    }

    fn stage_node(id: &str) -> NodeVersion {
        NodeVersion {
            id: id.into(),
            r#type: Some("ValueStreamStage".into()),
            props: Some(json!({ "name": id })),
        }
    }

    #[tokio::test]
    async fn commit_and_state_flow() {
        let engine = TemporalEngine::new().await.expect("engine");
        let commit_id = engine
            .commit(CommitChangesRequest {
                branch: "main".into(),
                parent: None,
                author: Some("tester".into()),
                time: None,
                message: "seed".into(),
                tags: vec![],
                changes: ChangeSet {
                    node_creates: vec![capability_node("cap-1")],
                    ..ChangeSet::default()
                },
            })
            .await
            .expect("commit ok");
        let result = engine
            .state_at(StateAtArgs {
                as_of: CommitRef::Id(commit_id),
                scenario: Some("main".into()),
                confidence: None,
            })
            .await
            .expect("state ok");
        assert!(result.nodes > 0);
        assert!(result.edges > 0);
    }

    #[tokio::test]
    async fn topology_delta_passthrough() {
        let engine = TemporalEngine::new().await.expect("engine");
        let base = engine
            .commit(CommitChangesRequest {
                branch: "main".into(),
                parent: None,
                author: None,
                time: None,
                message: "base".into(),
                tags: vec![],
                changes: ChangeSet {
                    node_creates: vec![capability_node("cap-root")],
                    ..ChangeSet::default()
                },
            })
            .await
            .expect("base commit");

        let expanded = engine
            .commit(CommitChangesRequest {
                branch: "main".into(),
                parent: Some(base.clone()),
                author: None,
                time: None,
                message: "expand".into(),
                tags: vec![],
                changes: {
                    let mut change = ChangeSet::default();
                    change.node_creates.push(stage_node("stage-extra"));
                    change.edge_creates.push(EdgeVersion {
                        id: None,
                        from: "cap-root".into(),
                        to: "stage-extra".into(),
                        r#type: Some("serves".into()),
                        directed: Some(true),
                        props: None,
                    });
                    change
                },
            })
            .await
            .expect("expanded commit");

        let delta = engine
            .topology_delta(TopologyDeltaArgs {
                from: CommitRef::Id(base.clone()),
                to: CommitRef::Id(expanded.clone()),
            })
            .await
            .expect("topology delta");
        assert_eq!(delta.node_adds, 1);
        assert_eq!(delta.edge_adds, 1);

        let trimmed = engine
            .commit(CommitChangesRequest {
                branch: "main".into(),
                parent: Some(expanded.clone()),
                author: None,
                time: None,
                message: "trim".into(),
                tags: vec![],
                changes: {
                    let mut change = ChangeSet::default();
                    change.edge_deletes.push(EdgeTombstone {
                        from: "cap-root".into(),
                        to: "stage-extra".into(),
                    });
                    change.node_deletes.push(NodeTombstone {
                        id: "stage-extra".into(),
                    });
                    change
                },
            })
            .await
            .expect("trim commit");

        let delta_trim = engine
            .topology_delta(TopologyDeltaArgs {
                from: CommitRef::Id(expanded),
                to: CommitRef::Id(trimmed),
            })
            .await
            .expect("topology delta trim");
        assert_eq!(delta_trim.node_dels, 1);
        assert_eq!(delta_trim.edge_dels, 1);
    }
}
