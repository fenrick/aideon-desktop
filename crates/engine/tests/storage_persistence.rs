use aideon_engine::temporal::{ChangeSet, CommitChangesRequest, NodeVersion};
use aideon_engine::{PraxisEngine, PraxisEngineConfig, SqliteDb, Store};
use serde_json::json;
use tempfile::tempdir;

#[tokio::test]
async fn sqlite_persists_commits_across_restarts() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("integration.sqlite");

    let engine = PraxisEngine::with_sqlite(&db_path)
        .await
        .expect("engine init");
    let parent = engine
        .list_commits("main".into())
        .await
        .expect("list commits")
        .last()
        .map(|commit| commit.id.clone());

    let commit_id = engine
        .commit(CommitChangesRequest {
            branch: "main".into(),
            parent,
            author: Some("health-check".into()),
            time: Some("2025-11-12T00:00:00Z".into()),
            message: "add capability for persistence test".into(),
            tags: vec!["test".into()],
            changes: ChangeSet {
                node_creates: vec![NodeVersion {
                    id: "cap-health".into(),
                    r#type: Some("Capability".into()),
                    props: Some(json!({ "name": "Health Check" })),
                }],
                ..ChangeSet::default()
            },
        })
        .await
        .expect("commit succeeds");

    drop(engine);

    let reopened = PraxisEngine::with_sqlite_unseeded(&db_path, PraxisEngineConfig::default())
        .await
        .expect("reopen engine");
    let commits = reopened
        .list_commits("main".into())
        .await
        .expect("list commits after restart");
    assert!(
        commits.iter().any(|summary| summary.id == commit_id),
        "persisted commit should still be present"
    );
    reopened
        .stats_for_commit(&commit_id)
        .await
        .expect("snapshot available for persisted commit");

    let db = SqliteDb::open(&db_path)
        .await
        .expect("open sqlite for tag check");
    let tag = format!("snapshot/{commit_id}");
    let resolved = db
        .get_tag(&tag)
        .await
        .expect("query tag")
        .expect("tag present");
    assert_eq!(resolved, commit_id, "snapshot tag should point to commit");
}
