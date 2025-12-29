use aideon_mneme_store::ops::{CreateEdgeInput, CreateNodeInput, SetPropIntervalInput};
use aideon_mneme_store::{
    ActorId, AnalyticsApi, EntityKind, ExportOpsInput, GraphReadApi, GraphWriteApi, Hlc, Id, Layer,
    ListEntitiesInput, MetamodelApi, MetamodelBatch, MnemeConfig, MnemeStore, PartitionId,
    PropertyWriteApi, ReadEntityAtTimeInput, SyncApi, TypeDef, TypeFieldDef, ValidTime, Value,
    ValueType,
};
use tempfile::tempdir;

fn new_ids() -> (PartitionId, ActorId) {
    (PartitionId(Id::new()), ActorId(Id::new()))
}

#[tokio::test]
async fn transaction_rolls_back_after_op_insert_failpoint() -> aideon_mneme_store::MnemeResult<()> {
    let dir = tempdir().expect("tempdir");
    let base = dir.path();
    let mut config = MnemeConfig::default_sqlite(base.join("mneme.sqlite").to_string_lossy());
    config.failpoints = Some(vec!["after_op_insert".to_string()]);
    let store = MnemeStore::connect(&config, base).await?;
    let (partition, actor) = new_ids();
    let node_id = Id::new();
    let result = store
        .create_node(CreateNodeInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            node_id,
            type_id: None,
            acl_group_id: None,
            owner_actor_id: None,
            visibility: None,
            write_options: None,
        })
        .await;
    assert!(result.is_err());
    let ops = store
        .export_ops(ExportOpsInput {
            partition,
            scenario_id: None,
            since_asserted_at: None,
            limit: 10,
        })
        .await?;
    assert!(ops.is_empty());
    Ok(())
}

#[tokio::test]
async fn transaction_rolls_back_after_property_fact_insert_failpoint()
-> aideon_mneme_store::MnemeResult<()> {
    let dir = tempdir().expect("tempdir");
    let base = dir.path();
    let mut config = MnemeConfig::default_sqlite(base.join("mneme.sqlite").to_string_lossy());
    config.failpoints = Some(vec!["after_property_fact_insert".to_string()]);
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

    let result = store
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
        .await;
    assert!(result.is_err());

    let read = store
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
    assert!(!read.properties.contains_key(&field_id));
    Ok(())
}

#[tokio::test]
async fn effective_schema_inheritance_applies_overrides() -> aideon_mneme_store::MnemeResult<()> {
    let dir = tempdir().expect("tempdir");
    let base = dir.path();
    let config = MnemeConfig::default_sqlite(base.join("schema.sqlite").to_string_lossy());
    let store = MnemeStore::connect(&config, base).await?;
    let (partition, actor) = new_ids();
    let base_type = Id::new();
    let child_type = Id::new();
    let field_id = Id::new();
    store
        .upsert_metamodel_batch(
            partition,
            actor,
            Hlc::now(),
            MetamodelBatch {
                types: vec![
                    TypeDef {
                        type_id: base_type,
                        applies_to: EntityKind::Node,
                        label: "Base".to_string(),
                        is_abstract: false,
                        parent_type_id: None,
                    },
                    TypeDef {
                        type_id: child_type,
                        applies_to: EntityKind::Node,
                        label: "Child".to_string(),
                        is_abstract: false,
                        parent_type_id: Some(base_type),
                    },
                ],
                fields: vec![aideon_mneme_store::FieldDef {
                    field_id,
                    label: "name".to_string(),
                    value_type: ValueType::Str,
                    cardinality_multi: false,
                    merge_policy: aideon_mneme_store::MergePolicy::Lww,
                    is_indexed: false,
                    disallow_overlap: false,
                }],
                type_fields: vec![
                    TypeFieldDef {
                        type_id: base_type,
                        field_id,
                        is_required: false,
                        default_value: Some(Value::Str("base".to_string())),
                        override_default: false,
                        tighten_required: false,
                        disallow_overlap: None,
                    },
                    TypeFieldDef {
                        type_id: child_type,
                        field_id,
                        is_required: true,
                        default_value: Some(Value::Str("child".to_string())),
                        override_default: true,
                        tighten_required: true,
                        disallow_overlap: None,
                    },
                ],
                edge_type_rules: Vec::new(),
                metamodel_version: None,
                metamodel_source: None,
            },
        )
        .await?;

    store
        .compile_effective_schema(partition, actor, Hlc::now(), child_type)
        .await?;
    let schema = store
        .get_effective_schema(partition, child_type)
        .await?
        .expect("schema");
    let field = schema
        .fields
        .iter()
        .find(|f| f.field_id == field_id)
        .expect("field");
    assert_eq!(field.default_value, Some(Value::Str("child".to_string())));
    assert!(field.is_required);
    Ok(())
}

#[tokio::test]
async fn read_entity_uses_effective_schema_defaults() -> aideon_mneme_store::MnemeResult<()> {
    let dir = tempdir().expect("tempdir");
    let base = dir.path();
    let config = MnemeConfig::default_sqlite(base.join("defaults.sqlite").to_string_lossy());
    let store = MnemeStore::connect(&config, base).await?;
    let (partition, actor) = new_ids();
    let base_type = Id::new();
    let child_type = Id::new();
    let field_id = Id::new();
    store
        .upsert_metamodel_batch(
            partition,
            actor,
            Hlc::now(),
            MetamodelBatch {
                types: vec![
                    TypeDef {
                        type_id: base_type,
                        applies_to: EntityKind::Node,
                        label: "Base".to_string(),
                        is_abstract: false,
                        parent_type_id: None,
                    },
                    TypeDef {
                        type_id: child_type,
                        applies_to: EntityKind::Node,
                        label: "Child".to_string(),
                        is_abstract: false,
                        parent_type_id: Some(base_type),
                    },
                ],
                fields: vec![aideon_mneme_store::FieldDef {
                    field_id,
                    label: "tier".to_string(),
                    value_type: ValueType::Str,
                    cardinality_multi: false,
                    merge_policy: aideon_mneme_store::MergePolicy::Lww,
                    is_indexed: false,
                    disallow_overlap: false,
                }],
                type_fields: vec![
                    TypeFieldDef {
                        type_id: base_type,
                        field_id,
                        is_required: false,
                        default_value: Some(Value::Str("base".to_string())),
                        override_default: false,
                        tighten_required: false,
                        disallow_overlap: None,
                    },
                    TypeFieldDef {
                        type_id: child_type,
                        field_id,
                        is_required: false,
                        default_value: Some(Value::Str("child".to_string())),
                        override_default: true,
                        tighten_required: false,
                        disallow_overlap: None,
                    },
                ],
                edge_type_rules: Vec::new(),
                metamodel_version: None,
                metamodel_source: None,
            },
        )
        .await?;

    store
        .compile_effective_schema(partition, actor, Hlc::now(), child_type)
        .await?;

    let node_id = Id::new();
    store
        .create_node(CreateNodeInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            node_id,
            type_id: Some(child_type),
            acl_group_id: None,
            owner_actor_id: None,
            visibility: None,
            write_options: None,
        })
        .await?;

    let read = store
        .read_entity_at_time(ReadEntityAtTimeInput {
            partition,
            scenario_id: None,
            security_context: None,
            entity_id: node_id,
            at_valid_time: ValidTime(1),
            as_of_asserted_at: None,
            field_ids: None,
            include_defaults: true,
        })
        .await?;

    match read.properties.get(&field_id) {
        Some(aideon_mneme_store::ReadValue::Single(Value::Str(value))) => {
            assert_eq!(value, "child");
        }
        other => panic!("unexpected default value {:?}", other),
    }
    Ok(())
}

#[tokio::test]
async fn edge_endpoints_are_immutable() -> aideon_mneme_store::MnemeResult<()> {
    let dir = tempdir().expect("tempdir");
    let base = dir.path();
    let config = MnemeConfig::default_sqlite(base.join("edges.sqlite").to_string_lossy());
    let store = MnemeStore::connect(&config, base).await?;
    let (partition, actor) = new_ids();
    let node_a = Id::new();
    let node_b = Id::new();
    let node_c = Id::new();
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
        .create_node(CreateNodeInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            node_id: node_c,
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
            layer: Layer::Actual,
            weight: None,
            acl_group_id: None,
            owner_actor_id: None,
            visibility: None,
            write_options: None,
        })
        .await?;

    let result = store
        .create_edge(CreateEdgeInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            edge_id,
            type_id: None,
            src_id: node_a,
            dst_id: node_c,
            exists_valid_from: ValidTime(0),
            exists_valid_to: None,
            layer: Layer::Actual,
            weight: None,
            acl_group_id: None,
            owner_actor_id: None,
            visibility: None,
            write_options: None,
        })
        .await;
    assert!(result.is_err());
    Ok(())
}

#[tokio::test]
async fn edge_type_rules_are_enforced() -> aideon_mneme_store::MnemeResult<()> {
    let dir = tempdir().expect("tempdir");
    let base = dir.path();
    let config = MnemeConfig::default_sqlite(base.join("edge-rules.sqlite").to_string_lossy());
    let store = MnemeStore::connect(&config, base).await?;
    let (partition, actor) = new_ids();
    let type_a = Id::new();
    let type_b = Id::new();
    let type_c = Id::new();
    let edge_type = Id::new();
    store
        .upsert_metamodel_batch(
            partition,
            actor,
            Hlc::now(),
            MetamodelBatch {
                types: vec![
                    TypeDef {
                        type_id: type_a,
                        applies_to: EntityKind::Node,
                        label: "A".to_string(),
                        is_abstract: false,
                        parent_type_id: None,
                    },
                    TypeDef {
                        type_id: type_b,
                        applies_to: EntityKind::Node,
                        label: "B".to_string(),
                        is_abstract: false,
                        parent_type_id: None,
                    },
                    TypeDef {
                        type_id: type_c,
                        applies_to: EntityKind::Node,
                        label: "C".to_string(),
                        is_abstract: false,
                        parent_type_id: None,
                    },
                    TypeDef {
                        type_id: edge_type,
                        applies_to: EntityKind::Edge,
                        label: "Rel".to_string(),
                        is_abstract: false,
                        parent_type_id: None,
                    },
                ],
                fields: Vec::new(),
                type_fields: Vec::new(),
                edge_type_rules: vec![aideon_mneme_store::EdgeTypeRule {
                    edge_type_id: edge_type,
                    allowed_src_type_ids: vec![type_a],
                    allowed_dst_type_ids: vec![type_b],
                    semantic_direction: Some("relates".to_string()),
                }],
                metamodel_version: None,
                metamodel_source: None,
            },
        )
        .await?;

    let node_a = Id::new();
    let node_b = Id::new();
    let node_c = Id::new();
    for (node_id, type_id) in [(node_a, type_a), (node_b, type_b), (node_c, type_c)] {
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
    }

    store
        .create_edge(CreateEdgeInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            edge_id: Id::new(),
            type_id: Some(edge_type),
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

    let result = store
        .create_edge(CreateEdgeInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            edge_id: Id::new(),
            type_id: Some(edge_type),
            src_id: node_c,
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
        .await;
    assert!(result.is_err());
    Ok(())
}

#[tokio::test]
async fn projection_edges_respect_valid_time() -> aideon_mneme_store::MnemeResult<()> {
    let dir = tempdir().expect("tempdir");
    let base = dir.path();
    let config = MnemeConfig::default_sqlite(base.join("projection.sqlite").to_string_lossy());
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
            exists_valid_from: ValidTime(10),
            exists_valid_to: Some(ValidTime(20)),
            layer: Layer::Actual,
            weight: None,
            acl_group_id: None,
            owner_actor_id: None,
            visibility: None,
            write_options: None,
        })
        .await?;

    let before = store
        .get_projection_edges(aideon_mneme_store::GetProjectionEdgesInput {
            partition,
            scenario_id: None,
            security_context: None,
            at_valid_time: Some(ValidTime(5)),
            as_of_asserted_at: None,
            edge_type_filter: None,
            limit: None,
        })
        .await?;
    assert!(before.is_empty());

    let during = store
        .get_projection_edges(aideon_mneme_store::GetProjectionEdgesInput {
            partition,
            scenario_id: None,
            security_context: None,
            at_valid_time: Some(ValidTime(15)),
            as_of_asserted_at: None,
            edge_type_filter: None,
            limit: None,
        })
        .await?;
    assert_eq!(during.len(), 1);
    assert_eq!(during[0].edge_id, edge_id);
    Ok(())
}

#[tokio::test]
async fn ingest_ops_is_idempotent() -> aideon_mneme_store::MnemeResult<()> {
    let dir = tempdir().expect("tempdir");
    let base = dir.path();
    let config = MnemeConfig::default_sqlite(base.join("ingest.sqlite").to_string_lossy());
    let store = MnemeStore::connect(&config, base).await?;
    let (partition, actor) = new_ids();
    let node_id = Id::new();
    store
        .create_node(CreateNodeInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            node_id,
            type_id: None,
            acl_group_id: None,
            owner_actor_id: None,
            visibility: None,
            write_options: None,
        })
        .await?;

    let ops = store
        .export_ops(ExportOpsInput {
            partition,
            scenario_id: None,
            since_asserted_at: None,
            limit: 10,
        })
        .await?;
    store.ingest_ops(partition, ops.clone()).await?;
    store.ingest_ops(partition, ops).await?;

    let list = store
        .list_entities(ListEntitiesInput {
            partition,
            scenario_id: None,
            security_context: None,
            kind: Some(EntityKind::Node),
            type_id: None,
            at_valid_time: ValidTime(1),
            as_of_asserted_at: None,
            filters: vec![],
            limit: 10,
            cursor: None,
        })
        .await?;
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].entity_id, node_id);
    Ok(())
}
