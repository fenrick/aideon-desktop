use aideon_mneme::ops::CreateNodeInput;
use aideon_mneme::{
    ActorId, ChangeFeedApi, GraphWriteApi, Hlc, Id, MnemeConfig, MnemeStore, PartitionId,
    ValidationRule, ValidationRulesApi,
};
use tempfile::tempdir;

#[tokio::test]
async fn change_feed_records_create_node() -> aideon_mneme::MnemeResult<()> {
    let dir = tempdir().expect("tempdir");
    let base = dir.path();
    let config = MnemeConfig::default_sqlite(base.join("mneme.sqlite").to_string_lossy());
    let store = MnemeStore::connect(&config, base).await?;
    let partition = PartitionId(Id::new());
    let actor = ActorId(Id::new());
    let node_id = Id::new();
    store
        .create_node(CreateNodeInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            node_id,
            type_id: None,
        })
        .await?;
    let changes = store.get_changes_since(partition, None, 10).await?;
    let event = changes
        .into_iter()
        .find(|evt| evt.entity_id == Some(node_id))
        .expect("expected change feed entry for node");
    assert_eq!(event.change_kind, 1);
    Ok(())
}

#[tokio::test]
async fn validation_rules_roundtrip() -> aideon_mneme::MnemeResult<()> {
    let dir = tempdir().expect("tempdir");
    let base = dir.path();
    let config = MnemeConfig::default_sqlite(base.join("mneme.sqlite").to_string_lossy());
    let store = MnemeStore::connect(&config, base).await?;
    let partition = PartitionId(Id::new());
    let actor = ActorId(Id::new());
    let rule = ValidationRule {
        rule_id: Id::new(),
        scope_kind: 1,
        scope_id: None,
        severity: 2,
        template_kind: "required".to_string(),
        params: serde_json::json!({"field": "name"}),
    };
    store
        .upsert_validation_rules(partition, actor, Hlc::now(), vec![rule.clone()])
        .await?;
    let rules = store.list_validation_rules(partition).await?;
    assert!(rules.contains(&rule));
    Ok(())
}
