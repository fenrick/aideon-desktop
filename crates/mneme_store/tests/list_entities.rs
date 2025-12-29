use aideon_mneme::{
    CompareOp, CreateNodeInput, EntityKind, FieldDef, FieldFilter, GraphReadApi, GraphWriteApi, Hlc,
    Id, Layer, ListEntitiesInput, MetamodelApi, MetamodelBatch, MnemeConfig, MnemeStore,
    PartitionId, PropertyWriteApi, SetPropIntervalInput, TypeDef, TypeFieldDef, ValidTime, Value,
    ValueType,
};
use tempfile::tempdir;

fn new_ids() -> (PartitionId, aideon_mneme::ActorId) {
    (PartitionId(Id::new()), aideon_mneme::ActorId(Id::new()))
}

#[tokio::test]
async fn list_entities_filters_indexed_fields() -> aideon_mneme::MnemeResult<()> {
    let dir = tempdir().expect("tempdir");
    let base = dir.path();
    let config = MnemeConfig::default_sqlite(base.join("mneme.sqlite").to_string_lossy());
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
                fields: vec![FieldDef {
                    field_id,
                    label: "name".to_string(),
                    value_type: ValueType::Str,
                    cardinality_multi: false,
                    merge_policy: aideon_mneme::MergePolicy::Lww,
                    is_indexed: true,
                }],
                type_fields: vec![TypeFieldDef {
                    type_id,
                    field_id,
                    is_required: false,
                    default_value: None,
                    override_default: false,
                    tighten_required: false,
                }],
                edge_type_rules: Vec::new(),
            },
        )
        .await?;

    let node_alpha = Id::new();
    let node_beta = Id::new();
    store
        .create_node(CreateNodeInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            node_id: node_alpha,
            type_id: Some(type_id),
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
            node_id: node_beta,
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
            entity_id: node_alpha,
            field_id,
            value: Value::Str("alpha".to_string()),
            valid_from: ValidTime(0),
            valid_to: None,
            layer: Layer::Actual,
            write_options: None,
        })
        .await?;
    store
        .set_property_interval(SetPropIntervalInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            entity_id: node_beta,
            field_id,
            value: Value::Str("beta".to_string()),
            valid_from: ValidTime(0),
            valid_to: None,
            layer: Layer::Actual,
            write_options: None,
        })
        .await?;

    let results = store
        .list_entities(ListEntitiesInput {
            partition,
            scenario_id: None,
            security_context: None,
            kind: Some(EntityKind::Node),
            type_id: Some(type_id),
            at_valid_time: ValidTime(1),
            as_of_asserted_at: None,
            filters: vec![FieldFilter {
                field_id,
                op: CompareOp::Eq,
                value: Value::Str("alpha".to_string()),
            }],
            limit: 10,
            cursor: None,
        })
        .await?;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].entity_id, node_alpha);
    Ok(())
}
