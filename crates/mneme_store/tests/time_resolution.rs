use aideon_mneme_store::{ActorId, Direction, Hlc, Id, MergePolicy, SyncApi, TraverseAtTimeInput};
use aideon_mneme_store::{
    CreateNodeInput, EntityKind, FieldDef, GraphReadApi, GraphWriteApi, Layer, MetamodelApi,
    MetamodelBatch, MnemeConfig, MnemeStore, PartitionId, PropertyWriteApi, ReadEntityAtTimeInput,
    SetEdgeExistenceIntervalInput, SetPropIntervalInput, TypeDef, TypeFieldDef, ValidTime, Value,
    ValueType,
};
use tempfile::tempdir;

fn new_ids() -> (PartitionId, ActorId) {
    (PartitionId(Id::new()), ActorId(Id::new()))
}

#[tokio::test]
async fn resolves_layer_precedence_for_lww() -> aideon_mneme_store::MnemeResult<()> {
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
                    label: "System".to_string(),
                    is_abstract: false,
                    parent_type_id: None,
                }],
                fields: vec![FieldDef {
                    field_id,
                    label: "name".to_string(),
                    value_type: ValueType::Str,
                    cardinality_multi: false,
                    merge_policy: MergePolicy::Lww,
                    is_indexed: true,
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
            value: Value::Str("planned".to_string()),
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
        .read_entity_at_time(ReadEntityAtTimeInput {
            partition,
            scenario_id: None,
            security_context: None,
            entity_id: node_id,
            at_valid_time: ValidTime(1),
            as_of_asserted_at: None,
            field_ids: Some(vec![field_id]),
            include_defaults: false,
        })
        .await?;
    let value = result.properties.get(&field_id).expect("field value");
    assert_eq!(
        value,
        &aideon_mneme_store::ReadValue::Single(Value::Str("actual".to_string()))
    );
    Ok(())
}

#[tokio::test]
async fn resolves_multi_value_for_mv() -> aideon_mneme_store::MnemeResult<()> {
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
                    label: "tag".to_string(),
                    value_type: ValueType::Str,
                    cardinality_multi: true,
                    merge_policy: MergePolicy::Mv,
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
            value: Value::Str("alpha".to_string()),
            valid_from: ValidTime(10),
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
            entity_id: node_id,
            field_id,
            value: Value::Str("beta".to_string()),
            valid_from: ValidTime(10),
            valid_to: None,
            layer: Layer::Actual,
            write_options: None,
        })
        .await?;
    let result = store
        .read_entity_at_time(ReadEntityAtTimeInput {
            partition,
            scenario_id: None,
            security_context: None,
            entity_id: node_id,
            at_valid_time: ValidTime(11),
            as_of_asserted_at: None,
            field_ids: Some(vec![field_id]),
            include_defaults: false,
        })
        .await?;
    let value = result.properties.get(&field_id).expect("field value");
    match value {
        aideon_mneme_store::ReadValue::Multi(values)
        | aideon_mneme_store::ReadValue::MultiLimited {
            values,
            more_available: _,
        } => {
            let mut labels: Vec<String> = values
                .iter()
                .filter_map(|v| match v {
                    Value::Str(s) => Some(s.clone()),
                    _ => None,
                })
                .collect();
            labels.sort();
            assert_eq!(labels, vec!["alpha".to_string(), "beta".to_string()]);
        }
        other => panic!("expected multi value, got {other:?}"),
    }
    Ok(())
}

#[tokio::test]
async fn respects_as_of_asserted_at_for_reads() -> aideon_mneme_store::MnemeResult<()> {
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
                    label: "System".to_string(),
                    is_abstract: false,
                    parent_type_id: None,
                }],
                fields: vec![FieldDef {
                    field_id,
                    label: "name".to_string(),
                    value_type: ValueType::Str,
                    cardinality_multi: false,
                    merge_policy: MergePolicy::Lww,
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
            value: Value::Str("alpha".to_string()),
            valid_from: ValidTime(0),
            valid_to: None,
            layer: Layer::Actual,
            write_options: None,
        })
        .await?;
    let as_of = store.get_partition_head(partition).await?;
    store
        .set_property_interval(SetPropIntervalInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            entity_id: node_id,
            field_id,
            value: Value::Str("beta".to_string()),
            valid_from: ValidTime(0),
            valid_to: None,
            layer: Layer::Actual,
            write_options: None,
        })
        .await?;
    let result = store
        .read_entity_at_time(ReadEntityAtTimeInput {
            partition,
            scenario_id: None,
            security_context: None,
            entity_id: node_id,
            at_valid_time: ValidTime(1),
            as_of_asserted_at: Some(as_of),
            field_ids: Some(vec![field_id]),
            include_defaults: false,
        })
        .await?;
    let value = result.properties.get(&field_id).expect("field value");
    assert_eq!(
        value,
        &aideon_mneme_store::ReadValue::Single(Value::Str("alpha".to_string()))
    );
    Ok(())
}

#[tokio::test]
async fn edge_existence_intervals_override_at_valid_time() -> aideon_mneme_store::MnemeResult<()> {
    let dir = tempdir().expect("tempdir");
    let base = dir.path();
    let config = MnemeConfig::default_sqlite(base.join("mneme.sqlite").to_string_lossy());
    let store = MnemeStore::connect(&config, base).await?;
    let (partition, actor) = new_ids();

    let node_a = Id::new();
    let node_b = Id::new();
    let edge_id = Id::new();
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
    store
        .create_edge(aideon_mneme_store::CreateEdgeInput {
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
            layer: Layer::Actual,
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
            valid_from: ValidTime(5),
            valid_to: None,
            layer: Layer::Actual,
            is_tombstone: true,
            write_options: None,
        })
        .await?;

    let edges_at_start = store
        .traverse_at_time(TraverseAtTimeInput {
            partition,
            scenario_id: None,
            security_context: None,
            from_entity_id: node_a,
            direction: Direction::Out,
            edge_type_id: None,
            at_valid_time: ValidTime(1),
            as_of_asserted_at: None,
            limit: 10,
        })
        .await?;
    assert_eq!(edges_at_start.len(), 1);

    let edges_after_tombstone = store
        .traverse_at_time(TraverseAtTimeInput {
            partition,
            scenario_id: None,
            security_context: None,
            from_entity_id: node_a,
            direction: Direction::Out,
            edge_type_id: None,
            at_valid_time: ValidTime(6),
            as_of_asserted_at: None,
            limit: 10,
        })
        .await?;
    assert!(edges_after_tombstone.is_empty());
    Ok(())
}

#[tokio::test]
async fn tombstone_edge_removes_traversal() -> aideon_mneme_store::MnemeResult<()> {
    let dir = tempdir().expect("tempdir");
    let base = dir.path();
    let config = MnemeConfig::default_sqlite(base.join("mneme.sqlite").to_string_lossy());
    let store = MnemeStore::connect(&config, base).await?;
    let (partition, actor) = new_ids();
    let node_type_id = Id::new();
    let edge_type_id = Id::new();
    store
        .upsert_metamodel_batch(
            partition,
            actor,
            Hlc::now(),
            MetamodelBatch {
                types: vec![
                    TypeDef {
                        type_id: node_type_id,
                        applies_to: EntityKind::Node,
                        label: "NodeType".to_string(),
                        is_abstract: false,
                        parent_type_id: None,
                    },
                    TypeDef {
                        type_id: edge_type_id,
                        applies_to: EntityKind::Edge,
                        label: "EdgeType".to_string(),
                        is_abstract: false,
                        parent_type_id: None,
                    },
                ],
                fields: Vec::new(),
                type_fields: Vec::new(),
                edge_type_rules: Vec::new(),
                metamodel_version: None,
                metamodel_source: None,
            },
        )
        .await?;
    let src_id = Id::new();
    let dst_id = Id::new();
    store
        .create_node(CreateNodeInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            node_id: src_id,
            type_id: Some(node_type_id),
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
            node_id: dst_id,
            type_id: Some(node_type_id),
            acl_group_id: None,
            owner_actor_id: None,
            visibility: None,
            write_options: None,
        })
        .await?;
    let edge_id = Id::new();
    store
        .create_edge(aideon_mneme_store::CreateEdgeInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            edge_id,
            type_id: None,
            src_id,
            dst_id,
            exists_valid_from: ValidTime(0),
            exists_valid_to: None,
            layer: Layer::Actual,
            weight: None,
            acl_group_id: None,
            owner_actor_id: None,
            visibility: None,
            write_options: None,
        })
        .await?;
    let edges = store
        .traverse_at_time(TraverseAtTimeInput {
            partition,
            scenario_id: None,
            security_context: None,
            from_entity_id: src_id,
            direction: Direction::Out,
            edge_type_id: None,
            at_valid_time: ValidTime(1),
            as_of_asserted_at: None,
            limit: 10,
        })
        .await?;
    assert_eq!(edges.len(), 1);
    store
        .tombstone_entity(partition, None, actor, Hlc::now(), edge_id)
        .await?;
    let edges = store
        .traverse_at_time(TraverseAtTimeInput {
            partition,
            scenario_id: None,
            security_context: None,
            from_entity_id: src_id,
            direction: Direction::Out,
            edge_type_id: None,
            at_valid_time: ValidTime(1),
            as_of_asserted_at: None,
            limit: 10,
        })
        .await?;
    assert!(edges.is_empty());
    Ok(())
}
