use aideon_praxis::PraxisEngine;
use aideon_praxis::temporal::{CommitChangesRequest, MergeRequest, StateAtArgs};
use serde_json::json;

#[tokio::test]
async fn merge_creates_commit_when_no_conflicts() {
    let engine = PraxisEngine::new().await.expect("engine init");

    // Prepare a source branch off main with one additional node.
    let base_head = engine
        .list_commits("main".into())
        .await
        .expect("list commits")
        .last()
        .map(|c| c.id.clone())
        .expect("baseline head");

    engine
        .create_branch(
            "feature/merge-demo".into(),
            Some(aideon_praxis::temporal::CommitRef::Id(base_head.clone())),
        )
        .await
        .expect("branch created");

    let change = CommitChangesRequest {
        branch: "feature/merge-demo".into(),
        parent: Some(base_head.clone()),
        author: Some("merge-flow".into()),
        time: Some("2025-11-28T00:00:00Z".into()),
        message: "add feature node".into(),
        tags: vec!["merge-demo".into()],
        changes: aideon_praxis::temporal::ChangeSet {
            node_creates: vec![aideon_praxis::temporal::NodeVersion {
                id: "feature-node".into(),
                r#type: Some("Capability".into()),
                props: Some(json!({"name":"Feature"})),
            }],
            ..Default::default()
        },
    };

    let feature_commit = engine.commit(change).await.expect("feature commit");

    // Merge feature back to main.
    let response = engine
        .merge(MergeRequest {
            source: "feature/merge-demo".into(),
            target: "main".into(),
        })
        .await
        .expect("merge ok");

    let merged_id = response.result.expect("merge commit id");
    let merged = engine
        .state_at(StateAtArgs::new(
            merged_id.clone(),
            Some("main".into()),
            None,
        ))
        .await
        .expect("merged state");

    assert!(merged.nodes > 0, "merged state should contain nodes");
    assert_eq!(merged.scenario.as_deref(), Some("main"));
    assert_ne!(merged.commit_id, base_head);
    assert!(
        merged.commit_id != feature_commit,
        "merge commit should differ from feature head"
    );
}

#[tokio::test]
async fn merge_returns_conflict_when_branches_diverge_on_same_node() {
    let engine = PraxisEngine::new().await.expect("engine init");
    let base_head = engine
        .list_commits("main".into())
        .await
        .expect("list commits")
        .last()
        .map(|c| c.id.clone())
        .expect("baseline head");

    // Seed a node on main so branches can diverge on it.
    let seeded_commit = engine
        .commit(CommitChangesRequest {
            branch: "main".into(),
            parent: Some(base_head.clone()),
            author: None,
            time: None,
            message: "seed conflict node".into(),
            tags: vec![],
            changes: aideon_praxis::temporal::ChangeSet {
                node_creates: vec![aideon_praxis::temporal::NodeVersion {
                    id: "conflict-node".into(),
                    r#type: Some("Capability".into()),
                    props: Some(json!({"name":"Seed"})),
                }],
                ..Default::default()
            },
        })
        .await
        .expect("seed commit");

    // Create two branches from the seeded base.
    engine
        .create_branch(
            "feature/alpha".into(),
            Some(aideon_praxis::temporal::CommitRef::Id(
                seeded_commit.clone(),
            )),
        )
        .await
        .expect("alpha branch");
    engine
        .create_branch(
            "feature/beta".into(),
            Some(aideon_praxis::temporal::CommitRef::Id(
                seeded_commit.clone(),
            )),
        )
        .await
        .expect("beta branch");

    // Alpha updates a node; Beta deletes it to force conflict.
    let target_node = aideon_praxis::temporal::NodeVersion {
        id: "conflict-node".into(),
        r#type: Some("Capability".into()),
        props: Some(json!({"name":"Conflicting"})),
    };

    engine
        .commit(CommitChangesRequest {
            branch: "feature/alpha".into(),
            parent: Some(seeded_commit.clone()),
            author: None,
            time: None,
            message: "alpha updates".into(),
            tags: vec![],
            changes: aideon_praxis::temporal::ChangeSet {
                node_updates: vec![target_node.clone()],
                ..Default::default()
            },
        })
        .await
        .expect("alpha commit");

    engine
        .commit(CommitChangesRequest {
            branch: "feature/beta".into(),
            parent: Some(seeded_commit.clone()),
            author: None,
            time: None,
            message: "beta deletes".into(),
            tags: vec![],
            changes: aideon_praxis::temporal::ChangeSet {
                node_deletes: vec![aideon_praxis::temporal::NodeTombstone {
                    id: target_node.id.clone(),
                }],
                ..Default::default()
            },
        })
        .await
        .expect("beta commit");

    let response = engine
        .merge(MergeRequest {
            source: "feature/alpha".into(),
            target: "feature/beta".into(),
        })
        .await
        .expect("merge completes");

    assert!(
        response.result.is_none(),
        "conflict should prevent auto-merge"
    );
    let conflicts = response.conflicts.expect("expected conflicts");
    assert!(!conflicts.is_empty());
}
