//! Utility and helper functions for the Praxis engine.

use crate::engine::state::Inner;
use crate::error::{PraxisError, PraxisResult};
use crate::graph::GraphSnapshot;
use crate::temporal::{ChangeSet, CommitRef, EdgeTombstone, EdgeVersion};
use blake3::Hasher;
use serde::Serialize;
use std::cmp::min;
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

pub(super) fn change_count(set: &ChangeSet) -> u64 {
    (set.node_creates.len()
        + set.node_updates.len()
        + set.node_deletes.len()
        + set.edge_creates.len()
        + set.edge_updates.len()
        + set.edge_deletes.len()) as u64
}

pub(super) fn normalize_change_set(input: &ChangeSet) -> ChangeSet {
    let mut normalized = input.clone();
    normalized.node_creates.sort_by_key(|node| node.id.clone());
    normalized.node_updates.sort_by_key(|node| node.id.clone());
    normalized.node_deletes.sort_by_key(|node| node.id.clone());
    normalized.edge_creates.sort_by_key(edge_sort_key);
    normalized.edge_updates.sort_by_key(edge_sort_key);
    normalized.edge_deletes.sort_by_key(edge_tombstone_key);
    normalized
}

fn edge_sort_key(edge: &EdgeVersion) -> (String, String, String) {
    (
        edge.id.clone().unwrap_or_default(),
        edge.from.clone(),
        edge.to.clone(),
    )
}

fn edge_tombstone_key(tombstone: &EdgeTombstone) -> (String, String) {
    (tombstone.from.clone(), tombstone.to.clone())
}

pub(super) fn derive_commit_id(
    prefix: &str,
    branch: &str,
    parents: &[String],
    author: Option<&str>,
    message: &str,
    tags: &[String],
    changes: &ChangeSet,
) -> String {
    #[derive(Serialize)]
    struct Identity<'a> {
        branch: &'a str,
        parents: &'a [String],
        author: Option<&'a str>,
        message: &'a str,
        tags: &'a [String],
        changes: &'a ChangeSet,
    }

    let identity = Identity {
        branch,
        parents,
        author,
        message,
        tags,
        changes,
    };
    let payload = serde_json::to_vec(&identity).expect("commit identity serialization");
    let mut hasher = Hasher::new();
    hasher.update(&payload);
    let hex = hasher.finalize().to_hex().to_string();
    let short = &hex[..min(32, hex.len())];
    format!("{}{}", prefix, short)
}

pub(super) fn current_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

pub(super) fn snapshot_tag(commit_id: &str) -> String {
    format!("snapshot/{commit_id}")
}

pub(super) fn validate_branch_name(name: &str) -> PraxisResult<()> {
    if name.trim().is_empty() {
        return Err(PraxisError::ValidationFailed {
            message: "branch name cannot be empty".into(),
        });
    }
    for segment in name.split('/') {
        if segment.is_empty() {
            return Err(PraxisError::ValidationFailed {
                message: "branch segments cannot be empty".into(),
            });
        }
        if segment == "." || segment == ".." {
            return Err(PraxisError::ValidationFailed {
                message: "branch segments may not be '.' or '..'".into(),
            });
        }
        if !segment
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        {
            return Err(PraxisError::ValidationFailed {
                message: format!("branch segment '{segment}' contains invalid characters"),
            });
        }
    }
    Ok(())
}

pub(super) async fn resolve_snapshot(
    inner: &mut Inner,
    reference: &CommitRef,
    scenario_hint: Option<&str>,
) -> PraxisResult<(String, Arc<GraphSnapshot>, String)> {
    let commit_id = resolve_commit_id(inner, reference, scenario_hint).await?;
    let record = inner.record_for(&commit_id).await?;
    Ok((
        commit_id,
        Arc::clone(&record.snapshot),
        record.summary.branch.clone(),
    ))
}

pub(super) async fn resolve_commit_id(
    inner: &mut Inner,
    reference: &CommitRef,
    scenario_hint: Option<&str>,
) -> PraxisResult<String> {
    match reference {
        CommitRef::Id(value) => {
            if inner.record_for(value).await.is_ok() {
                Ok(value.clone())
            } else if let Some(branch_state) = inner.branches.get(value.as_str()) {
                branch_state
                    .head
                    .clone()
                    .ok_or_else(|| PraxisError::UnknownCommit {
                        commit: value.clone(),
                    })
            } else if let Some(hint) = scenario_hint {
                let branch =
                    inner
                        .branches
                        .get(hint)
                        .ok_or_else(|| PraxisError::UnknownBranch {
                            branch: hint.into(),
                        })?;
                branch
                    .head
                    .clone()
                    .ok_or_else(|| PraxisError::UnknownCommit {
                        commit: hint.into(),
                    })
            } else {
                Err(PraxisError::UnknownCommit {
                    commit: value.clone(),
                })
            }
        }
        CommitRef::Branch { branch, at } => {
            if let Some(at) = at {
                inner.record_for(at).await?;
                return Ok(at.clone());
            }
            let target_branch =
                inner
                    .branches
                    .get(branch)
                    .ok_or_else(|| PraxisError::UnknownBranch {
                        branch: branch.clone(),
                    })?;
            target_branch
                .head
                .clone()
                .ok_or_else(|| PraxisError::UnknownCommit {
                    commit: branch.clone(),
                })
        }
    }
}

pub(super) async fn find_common_ancestor(
    inner: &mut Inner,
    a: &str,
    b: &str,
) -> PraxisResult<Option<String>> {
    let ancestors_a = collect_ancestors(inner, a).await?;
    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(b.to_string());
    let mut visited = HashSet::new();
    while let Some(id) = queue.pop_front() {
        if !visited.insert(id.clone()) {
            continue;
        }
        if ancestors_a.contains(&id) {
            return Ok(Some(id));
        }
        if let Ok(record) = inner.record_for(&id).await {
            for parent in &record.summary.parents {
                queue.push_back(parent.clone());
            }
        }
    }
    Ok(None)
}

async fn collect_ancestors(inner: &mut Inner, head: &str) -> PraxisResult<HashSet<String>> {
    let mut visited = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(head.to_string());
    while let Some(id) = queue.pop_front() {
        if !visited.insert(id.clone()) {
            continue;
        }
        if let Ok(record) = inner.record_for(&id).await {
            for parent in &record.summary.parents {
                queue.push_back(parent.clone());
            }
        }
    }
    Ok(visited)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporal::{EdgeVersion, NodeTombstone, NodeVersion};

    #[test]
    fn change_count_counts_all_change_vectors() {
        let mut set = ChangeSet::default();
        set.node_creates.push(NodeVersion {
            id: "n1".into(),
            r#type: None,
            props: None,
        });
        set.node_updates.push(NodeVersion {
            id: "n2".into(),
            r#type: None,
            props: None,
        });
        set.node_deletes.push(NodeTombstone { id: "n3".into() });
        set.edge_creates.push(EdgeVersion {
            id: Some("e1".into()),
            from: "n1".into(),
            to: "n2".into(),
            r#type: None,
            directed: None,
            props: None,
        });
        assert_eq!(change_count(&set), 4);
    }

    #[test]
    fn normalize_change_set_sorts_by_id_and_endpoints() {
        let mut set = ChangeSet::default();
        set.node_creates.push(NodeVersion {
            id: "b".into(),
            r#type: None,
            props: None,
        });
        set.node_creates.push(NodeVersion {
            id: "a".into(),
            r#type: None,
            props: None,
        });
        set.edge_creates.push(EdgeVersion {
            id: Some("2".into()),
            from: "b".into(),
            to: "c".into(),
            r#type: None,
            directed: None,
            props: None,
        });
        set.edge_creates.push(EdgeVersion {
            id: Some("1".into()),
            from: "a".into(),
            to: "c".into(),
            r#type: None,
            directed: None,
            props: None,
        });

        let normalized = normalize_change_set(&set);
        assert_eq!(normalized.node_creates[0].id, "a");
        assert_eq!(normalized.node_creates[1].id, "b");
        assert_eq!(normalized.edge_creates[0].id.as_deref(), Some("1"));
        assert_eq!(normalized.edge_creates[1].id.as_deref(), Some("2"));
    }

    #[test]
    fn derive_commit_id_is_deterministic_for_same_input() {
        let set = ChangeSet::default();
        let id1 = derive_commit_id(
            "commit-",
            "main",
            &["p1".into()],
            Some("me"),
            "message",
            &["tag".into()],
            &set,
        );
        let id2 = derive_commit_id(
            "commit-",
            "main",
            &["p1".into()],
            Some("me"),
            "message",
            &["tag".into()],
            &set,
        );
        assert_eq!(id1, id2);
        assert!(id1.starts_with("commit-"));
    }

    #[test]
    fn snapshot_tag_is_prefixed() {
        assert_eq!(snapshot_tag("abc"), "snapshot/abc");
    }

    #[test]
    fn validate_branch_name_rejects_empty_or_invalid_segments() {
        assert!(validate_branch_name("").is_err());
        assert!(validate_branch_name(" ").is_err());
        assert!(validate_branch_name("a//b").is_err());
        assert!(validate_branch_name("a/../b").is_err());
        assert!(validate_branch_name("a/b@c").is_err());
        validate_branch_name("feature/test_1").unwrap();
        validate_branch_name("release-1.0").unwrap();
    }
}
