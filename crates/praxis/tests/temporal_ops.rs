use aideon_praxis::PraxisEngine;
use aideon_praxis::temporal::{
    ChangeSet, CommitChangesRequest, DiffArgs, NodeVersion, StateAtArgs,
};
use serde_json::json;

#[tokio::test]
async fn state_at_and_diff_cover_new_commit() {
    let engine = PraxisEngine::new().await.expect("engine init");

    let baseline_head = engine
        .list_commits("main".into())
        .await
        .expect("list commits")
        .last()
        .map(|c| c.id.clone())
        .expect("baseline head");

    let baseline_state = engine
        .state_at(StateAtArgs::new(
            baseline_head.clone(),
            Some("main".into()),
            None,
        ))
        .await
        .expect("baseline state_at");

    let commit_id = engine
        .commit(CommitChangesRequest {
            branch: "main".into(),
            parent: Some(baseline_head.clone()),
            author: Some("temporal-test".into()),
            time: Some("2025-11-28T00:00:00Z".into()),
            message: "add node for temporal coverage".into(),
            tags: vec!["test".into()],
            changes: ChangeSet {
                node_creates: vec![NodeVersion {
                    id: "cap-temporal-coverage".into(),
                    r#type: Some("Capability".into()),
                    props: Some(json!({ "name": "Temporal Coverage" })),
                }],
                ..ChangeSet::default()
            },
        })
        .await
        .expect("commit succeeds");

    let updated_state = engine
        .state_at(StateAtArgs::new(
            commit_id.clone(),
            Some("main".into()),
            None,
        ))
        .await
        .expect("updated state_at");

    assert!(updated_state.nodes > baseline_state.nodes);
    assert_eq!(updated_state.scenario.as_deref(), Some("main"));

    let diff = engine
        .diff_summary(DiffArgs {
            from: aideon_praxis::temporal::CommitRef::Id(baseline_head.clone()),
            to: aideon_praxis::temporal::CommitRef::Id(commit_id.clone()),
            scope: None,
        })
        .await
        .expect("diff summary");

    assert_eq!(diff.from, baseline_head);
    assert_eq!(diff.to, commit_id);
    assert_eq!(diff.node_adds, 1, "expected one node addition in diff");
}
