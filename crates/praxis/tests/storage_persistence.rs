use aideon_praxis::temporal::{ChangeSet, CommitChangesRequest, NodeVersion};
use aideon_praxis::{PraxisEngine, PraxisEngineConfig, SqliteDb, Store};
use rusqlite::Connection;
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

#[tokio::test]
async fn sqlite_migrates_legacy_commit_schema() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("legacy.sqlite");

    {
        let conn = Connection::open(&db_path).expect("open legacy sqlite");
        conn.execute_batch(
            "BEGIN;
             CREATE TABLE commits (
                 commit_id TEXT PRIMARY KEY,
                 summary_json TEXT NOT NULL
             );
             COMMIT;",
        )
        .expect("create legacy commits table");

        let summary = json!({
            "id": "c-legacy-1",
            "parents": [],
            "branch": "main",
            "author": "legacy",
            "time": "2025-11-12T00:00:00Z",
            "message": "legacy commit",
            "tags": [],
            "changeCount": 0
        });
        conn.execute(
            "INSERT INTO commits (commit_id, summary_json) VALUES (?1, ?2)",
            ("c-legacy-1", summary.to_string()),
        )
        .expect("insert legacy commit");
    }

    let db = SqliteDb::open(&db_path)
        .await
        .expect("open sqlite with migration");
    let commit = db
        .get_commit("c-legacy-1")
        .await
        .expect("load migrated commit")
        .expect("commit present");
    assert_eq!(commit.summary.id, "c-legacy-1");
    assert!(commit.change_set.is_empty());
}

#[tokio::test]
async fn sqlite_seeding_is_idempotent() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("seed.sqlite");

    let engine = PraxisEngine::with_sqlite_unseeded(&db_path, PraxisEngineConfig::default())
        .await
        .expect("engine init");
    engine.ensure_seeded().await.expect("first seed");
    engine.ensure_seeded().await.expect("second seed");

    let commits = engine
        .list_commits("main".into())
        .await
        .expect("list commits");
    assert!(!commits.is_empty());
}
