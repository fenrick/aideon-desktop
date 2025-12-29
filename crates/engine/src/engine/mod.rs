//! The core Praxis engine for temporal graph operations.

use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};

use crate::meta::MetaModelDocument;
use crate::temporal::{
    BranchInfo, CommitChangesRequest, CommitRef, CommitSummary, DiffArgs, DiffSummary,
    MergeRequest, MergeResponse, StateAtArgs, StateAtResult, TopologyDeltaArgs,
    TopologyDeltaResult,
};

use crate::error::PraxisResult;
use crate::graph::GraphSnapshot;

mod config;
mod init;
mod ops;
mod seed;
mod state;
mod util;

pub use config::PraxisEngineConfig;
use state::Inner;

#[derive(Clone)]
pub struct PraxisEngine {
    inner: Arc<Mutex<Inner>>,
}

impl PraxisEngine {
    async fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().await
    }

    pub async fn commit(&self, request: CommitChangesRequest) -> PraxisResult<String> {
        let mut guard = self.lock().await;
        ops::commit(&mut guard, request).await
    }

    pub async fn create_branch(
        &self,
        name: String,
        from: Option<CommitRef>,
    ) -> PraxisResult<BranchInfo> {
        let mut guard = self.lock().await;
        ops::create_branch(&mut guard, name, from).await
    }

    pub async fn list_commits(&self, branch: String) -> PraxisResult<Vec<CommitSummary>> {
        let mut guard = self.lock().await;
        ops::list_commits(&mut guard, branch).await
    }

    pub async fn state_at(&self, args: StateAtArgs) -> PraxisResult<StateAtResult> {
        let mut guard = self.lock().await;
        ops::state_at(&mut guard, args).await
    }

    pub async fn diff_summary(&self, args: DiffArgs) -> PraxisResult<DiffSummary> {
        let mut guard = self.lock().await;
        ops::diff_summary(&mut guard, args).await
    }

    pub async fn topology_delta(
        &self,
        args: TopologyDeltaArgs,
    ) -> PraxisResult<TopologyDeltaResult> {
        let mut guard = self.lock().await;
        ops::topology_delta(&mut guard, args).await
    }

    pub async fn list_branches(&self) -> Vec<BranchInfo> {
        let guard = self.lock().await;
        guard
            .branches
            .iter()
            .map(|(name, state)| BranchInfo {
                name: name.clone(),
                head: state.head.clone(),
            })
            .collect()
    }

    pub async fn stats_for_commit(
        &self,
        commit_id: &str,
    ) -> PraxisResult<crate::graph::SnapshotStats> {
        let mut guard = self.lock().await;
        let record = guard.record_for(commit_id).await?;
        Ok(record.snapshot.stats())
    }

    pub async fn snapshot_for_commit(&self, commit_id: &str) -> PraxisResult<Arc<GraphSnapshot>> {
        let mut guard = self.lock().await;
        guard.snapshot_for(commit_id).await
    }

    pub async fn meta_model(&self) -> MetaModelDocument {
        let guard = self.lock().await;
        guard.registry.document()
    }

    pub async fn merge(&self, request: MergeRequest) -> PraxisResult<MergeResponse> {
        let mut guard = self.lock().await;
        ops::merge(&mut guard, request).await
    }
}
