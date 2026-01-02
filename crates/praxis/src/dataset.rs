use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::temporal::{ChangeSet, EdgeVersion, NodeVersion};
use serde::Deserialize;
use serde_json::Value;

use crate::error::{PraxisError, PraxisResult};

const BASELINE_YAML: &str = include_str!("../../../docs/data/base/baseline.yaml");

#[derive(Debug, Clone)]
pub struct BaselineDataset {
    pub version: String,
    pub description: Option<String>,
    pub released: Option<String>,
    commits: Vec<DatasetCommit>,
}

#[derive(Debug, Clone)]
pub struct DatasetCommit {
    pub key: String,
    pub branch: String,
    pub author: Option<String>,
    pub time: Option<String>,
    pub message: String,
    pub tags: Vec<String>,
    pub changes: ChangeSet,
}

impl BaselineDataset {
    pub fn embedded() -> PraxisResult<Self> {
        Self::from_str(BASELINE_YAML)
    }

    pub fn from_path(path: impl AsRef<Path>) -> PraxisResult<Self> {
        let raw = fs::read_to_string(&path).map_err(|err| PraxisError::IntegrityViolation {
            message: format!(
                "failed to read dataset '{}': {err}",
                path.as_ref().display()
            ),
        })?;
        Self::from_str(&raw)
    }

    fn from_str(raw: &str) -> PraxisResult<Self> {
        let spec: DatasetSpec =
            serde_yaml::from_str(raw).map_err(|err| PraxisError::IntegrityViolation {
                message: format!("baseline dataset parse error: {err}"),
            })?;
        spec.prepare()
    }

    pub fn commits(&self) -> &[DatasetCommit] {
        &self.commits
    }
}

impl DatasetCommit {
    pub fn to_request(&self, parent: Option<String>) -> crate::temporal::CommitChangesRequest {
        crate::temporal::CommitChangesRequest {
            branch: self.branch.clone(),
            parent,
            author: self.author.clone(),
            time: self.time.clone(),
            message: self.message.clone(),
            tags: self.tags.clone(),
            changes: self.changes.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct DatasetSpec {
    version: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    released: Option<String>,
    #[serde(default)]
    defaults: Option<DatasetDefaults>,
    commits: Vec<DatasetCommitSpec>,
}

#[derive(Debug, Deserialize, Default)]
struct DatasetDefaults {
    #[serde(default = "default_branch")]
    branch: String,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    time: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct DatasetCommitSpec {
    key: String,
    #[serde(default)]
    branch: Option<String>,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    time: Option<String>,
    message: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    nodes: Vec<NodeSpec>,
    #[serde(default)]
    edges: Vec<EdgeSpec>,
}

#[derive(Debug, Deserialize)]
struct NodeSpec {
    id: String,
    #[serde(rename = "type")]
    node_type: String,
    #[serde(default)]
    props: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct EdgeSpec {
    #[serde(default)]
    id: Option<String>,
    from: String,
    to: String,
    #[serde(rename = "type")]
    rel_type: String,
    #[serde(default)]
    directed: Option<bool>,
    #[serde(default)]
    props: Option<Value>,
}

impl DatasetSpec {
    fn prepare(self) -> PraxisResult<BaselineDataset> {
        if self.commits.is_empty() {
            return Err(PraxisError::IntegrityViolation {
                message: "baseline dataset missing commits".into(),
            });
        }
        let defaults = self.defaults.unwrap_or_default();
        let commits = self
            .commits
            .into_iter()
            .map(|spec| spec.into_commit(&defaults))
            .collect::<PraxisResult<Vec<_>>>()?;
        Ok(BaselineDataset {
            version: self.version,
            description: self.description,
            released: self.released,
            commits,
        })
    }
}

impl DatasetCommitSpec {
    fn into_commit(self, defaults: &DatasetDefaults) -> PraxisResult<DatasetCommit> {
        if self.nodes.is_empty() && self.edges.is_empty() {
            return Err(PraxisError::IntegrityViolation {
                message: format!("dataset commit '{}' has no nodes or edges", self.key),
            });
        }
        let mut tags = Vec::new();
        let mut seen = HashSet::new();
        for tag in defaults.tags.iter().chain(self.tags.iter()) {
            if seen.insert(tag.clone()) {
                tags.push(tag.clone());
            }
        }

        let node_creates = self
            .nodes
            .into_iter()
            .map(|spec| NodeVersion {
                id: spec.id,
                r#type: Some(spec.node_type),
                props: spec.props,
            })
            .collect();
        let edge_creates = self
            .edges
            .into_iter()
            .map(|spec| EdgeVersion {
                id: spec.id,
                from: spec.from,
                to: spec.to,
                r#type: Some(spec.rel_type),
                directed: spec.directed,
                props: spec.props,
            })
            .collect();

        let changes = ChangeSet {
            node_creates,
            edge_creates,
            ..Default::default()
        };

        Ok(DatasetCommit {
            key: self.key,
            branch: self.branch.unwrap_or_else(|| defaults.branch.clone()),
            author: self.author.or_else(|| defaults.author.clone()),
            time: self.time.or_else(|| defaults.time.clone()),
            message: self.message,
            tags,
            changes,
        })
    }
}

fn default_branch() -> String {
    "main".into()
}

#[cfg(test)]
mod tests {
    use crate::store::{MemoryStore, Store};
    use crate::temporal::ChangeSet;
    use std::sync::Arc;

    use super::*;
    use crate::{PraxisEngine, PraxisEngineConfig};

    #[test]
    fn loads_embedded_dataset() {
        let dataset = BaselineDataset::embedded().expect("dataset");
        assert_eq!(dataset.version, "1.0.0");
        assert!(!dataset.commits().is_empty());
        assert!(matches!(dataset.commits()[0].changes, ChangeSet { .. }));
    }

    #[tokio::test]
    async fn dataset_bootstrap_yields_expected_counts() {
        let dataset = BaselineDataset::embedded().expect("dataset");
        let store: Arc<dyn Store> = Arc::new(MemoryStore::default());
        let engine = PraxisEngine::with_stores_unseeded(PraxisEngineConfig::default(), store)
            .await
            .expect("engine");
        engine
            .bootstrap_with_dataset(&dataset)
            .await
            .expect("bootstrap");

        let commits = engine
            .list_commits("main".into())
            .await
            .expect("list commits");
        assert_eq!(
            commits.len(),
            dataset.commits().len() + 1,
            "meta seed + dataset commits expected",
        );
        let stats = engine
            .stats_for_commit(&commits.last().expect("head").id)
            .await
            .expect("stats");
        assert!(stats.node_count >= 20, "expected rich baseline nodes");
        assert!(stats.edge_count >= 14, "expected baseline edges");
    }
}
