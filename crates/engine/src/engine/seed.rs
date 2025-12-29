//! Praxis engine data seeding and bootstrapping.

use crate::PraxisEngine;
use crate::dataset::BaselineDataset;
use crate::error::PraxisResult;
use crate::meta_seed::meta_model_seed_change_set;
use crate::temporal::CommitChangesRequest;
use std::collections::BTreeMap;

impl PraxisEngine {
    /// Ensure the commit log contains an initial design sample commit.
    pub async fn ensure_seeded(&self) -> PraxisResult<()> {
        let needs_seed = {
            let inner = self.lock().await;
            inner
                .branches
                .get("main")
                .and_then(|branch| branch.head.clone())
                .is_none()
        };

        if !needs_seed {
            return Ok(());
        }

        let dataset = BaselineDataset::embedded()?;
        self.bootstrap_with_dataset(&dataset).await?;
        Ok(())
    }

    pub async fn bootstrap_with_dataset(&self, dataset: &BaselineDataset) -> PraxisResult<()> {
        self.seed_meta_commit().await?;
        self.apply_dataset_commits(dataset).await?;
        Ok(())
    }

    async fn seed_meta_commit(&self) -> PraxisResult<String> {
        let meta_changes = meta_model_seed_change_set();
        let request = CommitChangesRequest {
            branch: "main".into(),
            parent: None,
            author: Some("bootstrap".into()),
            time: None,
            message: "seed: meta-model".into(),
            tags: vec!["baseline".into(), "meta".into()],
            changes: meta_changes,
        };
        self.commit(request).await
    }

    async fn apply_dataset_commits(&self, dataset: &BaselineDataset) -> PraxisResult<()> {
        let mut branch_heads: BTreeMap<String, Option<String>> = self
            .list_branches()
            .await
            .into_iter()
            .map(|info| (info.name.clone(), info.head.clone()))
            .collect();
        for commit in dataset.commits() {
            let branch = commit.branch.clone();
            let parent = branch_heads.get(&branch).and_then(|head| head.clone());
            let request = commit.to_request(parent);
            let next_id = self.commit(request).await?;
            branch_heads.insert(branch, Some(next_id));
        }
        Ok(())
    }
}
