use aideon_mneme_store::ops::{
    CreateEdgeInput, CreateNodeInput, SetEdgeExistenceIntervalInput, SetPropIntervalInput,
};
use aideon_mneme_store::{
    ActorId, DiagnosticsApi, EntityKind, ExplainResolutionInput, ExplainTraversalInput,
    GraphWriteApi, Hlc, Id, Layer, MetamodelApi, MetamodelBatch, MnemeConfig, MnemeStore,
    PartitionId, PropertyWriteApi, ReadValue, TypeDef, TypeFieldDef, ValidTime, Value, ValueType,
};
use tempfile::tempdir;

fn new_ids() -> (PartitionId, ActorId) {
    (PartitionId(Id::new()), ActorId(Id::new()))
}

#[tokio::test]
async fn explain_resolution_prefers_higher_layer() -> aideon_mneme_store::MnemeResult<()> {
    let dir = tempdir().expect("tempdir");
    let base = dir.path();
    let config = MnemeConfig::default_sqlite(base.join("explain.sqlite").to_string_lossy());
    let store = MnemeStore::connect(&config, base).await?;
    let (partition, actor) = new_ids();

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
                fields: vec![aideon_mneme_store::FieldDef {
                    field_id,
                    label: "name".to_string(),
                    value_type: ValueType::Str,
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
                edge_type_rules: Vec::new(),
                metamodel_version: None,
                metamodel_source: None,
            },
        )
        .await?;

    let node_id = Id::new();
    store
        .create_node(CreateNodeInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            node_id,
            type_id: Some(type_id),
            acl_group_id: None,
            owner_actor_id: None,
            visibility: None,
            write_options: None,
        })
        .await?;

    store
        .set_property_interval(SetPropIntervalInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            entity_id: node_id,
            field_id,
            value: Value::Str("plan".to_string()),
            valid_from: ValidTime(0),
            valid_to: None,
            layer: Layer::Plan,
            write_options: None,
        })
        .await?;
    store
        .set_property_interval(SetPropIntervalInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            entity_id: node_id,
            field_id,
            value: Value::Str("actual".to_string()),
            valid_from: ValidTime(0),
            valid_to: None,
            layer: Layer::Actual,
            write_options: None,
        })
        .await?;

    let result = store
        .explain_resolution(ExplainResolutionInput {
            partition,
            scenario_id: None,
            security_context: None,
            entity_id: node_id,
            field_id,
            at_valid_time: ValidTime(1),
            as_of_asserted_at: None,
        })
        .await?;

    match result.resolved {
        Some(ReadValue::Single(Value::Str(value))) => assert_eq!(value, "actual"),
        other => panic!("unexpected resolved value {other:?}"),
    }
    let winner = result.winner.expect("winner");
    assert_eq!(winner.layer, Layer::Actual as i64);
    assert!(result.candidates.len() >= 2);
    Ok(())
}

#[tokio::test]
async fn explain_traversal_prefers_higher_layer() -> aideon_mneme_store::MnemeResult<()> {
    let dir = tempdir().expect("tempdir");
    let base = dir.path();
    let config = MnemeConfig::default_sqlite(base.join("explain_edges.sqlite").to_string_lossy());
    let store = MnemeStore::connect(&config, base).await?;
    let (partition, actor) = new_ids();

    let node_a = Id::new();
    let node_b = Id::new();
    store
        .create_node(CreateNodeInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            node_id: node_a,
            type_id: None,
            acl_group_id: None,
            owner_actor_id: None,
            visibility: None,
            write_options: None,
        })
        .await?;
    store
        .create_node(CreateNodeInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            node_id: node_b,
            type_id: None,
            acl_group_id: None,
            owner_actor_id: None,
            visibility: None,
            write_options: None,
        })
        .await?;

    let edge_id = Id::new();
    store
        .create_edge(CreateEdgeInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            edge_id,
            type_id: None,
            src_id: node_a,
            dst_id: node_b,
            exists_valid_from: ValidTime(0),
            exists_valid_to: None,
            layer: Layer::Plan,
            weight: None,
            acl_group_id: None,
            owner_actor_id: None,
            visibility: None,
            write_options: None,
        })
        .await?;
    store
        .set_edge_existence_interval(SetEdgeExistenceIntervalInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            edge_id,
            valid_from: ValidTime(0),
            valid_to: None,
            layer: Layer::Actual,
            is_tombstone: false,
            write_options: None,
        })
        .await?;

    let result = store
        .explain_traversal(ExplainTraversalInput {
            partition,
            scenario_id: None,
            security_context: None,
            edge_id,
            at_valid_time: ValidTime(1),
            as_of_asserted_at: None,
        })
        .await?;
    assert!(result.active);
    let winner = result.winner.expect("winner");
    assert_eq!(winner.layer, Layer::Actual as i64);
    assert!(result.candidates.len() >= 2);
    Ok(())
}
