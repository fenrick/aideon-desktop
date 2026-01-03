use aideon_mneme_store::{
    ActorId, ComputedCacheApi, ComputedCacheEntry, ComputedRule, ComputedRulesApi, CreateNodeInput,
    EntityKind, FieldDef, GraphWriteApi, Hlc, Id, ListComputedCacheInput, MetamodelApi,
    MetamodelBatch, MnemeConfig, MnemeStore, PartitionId, TypeDef, TypeFieldDef, ValidTime, Value,
    ValueType,
};
use tempfile::tempdir;

#[tokio::test]
async fn computed_rules_and_cache_roundtrip() -> aideon_mneme_store::MnemeResult<()> {
    let dir = tempdir().expect("tempdir");
    let base = dir.path();
    let config = MnemeConfig::default_sqlite(base.join("computed.sqlite").to_string_lossy());
    let store = MnemeStore::connect(&config, base).await?;
    let partition = PartitionId(Id::new());
    let actor = ActorId(Id::new());
    let type_id = Id::new();
    let field_id = Id::new();

    store
        .upsert_metamodel_batch(
            partition,
            actor,
            Hlc::now(),
            MetamodelBatch {
                types: vec![TypeDef {
                    type_id,
                    applies_to: EntityKind::Node,
                    label: "Service".to_string(),
                    is_abstract: false,
                    parent_type_id: None,
                }],
                fields: vec![FieldDef {
                    field_id,
                    label: "score".to_string(),
                    value_type: ValueType::I64,
                    cardinality_multi: false,
                    merge_policy: aideon_mneme_store::MergePolicy::Lww,
                    is_indexed: false,
                    disallow_overlap: false,
                }],
                type_fields: vec![TypeFieldDef {
                    type_id,
                    field_id,
                    is_required: false,
                    default_value: None,
                    override_default: false,
                    tighten_required: false,
                    disallow_overlap: None,
                }],
                edge_type_rules: vec![],
                metamodel_version: None,
                metamodel_source: None,
            },
        )
        .await?;

    store
        .upsert_computed_rules(
            partition,
            actor,
            Hlc::now(),
            vec![ComputedRule {
                rule_id: Id::new(),
                target_type_id: Some(type_id),
                output_field_id: Some(field_id),
                template_kind: "score:constant".to_string(),
                params: serde_json::json!({ "value": 42 }),
            }],
        )
        .await?;

    let rules = store.list_computed_rules(partition).await?;
    assert_eq!(rules.len(), 1);

    let entity_id = Id::new();
    store
        .create_node(CreateNodeInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            node_id: entity_id,
            type_id: Some(type_id),
            acl_group_id: None,
            owner_actor_id: None,
            visibility: None,
            write_options: None,
        })
        .await?;

    store
        .upsert_computed_cache(
            partition,
            vec![ComputedCacheEntry {
                entity_id,
                field_id,
                valid_from: 0,
                valid_to: Some(10),
                value: Value::I64(100),
                rule_version_hash: "v1".to_string(),
                computed_asserted_at: Hlc::now(),
            }],
        )
        .await?;

    let entries = store
        .list_computed_cache(ListComputedCacheInput {
            partition,
            entity_id: Some(entity_id),
            field_id,
            at_valid_time: Some(ValidTime(5)),
            limit: 10,
        })
        .await?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].value, Value::I64(100));
    Ok(())
}
