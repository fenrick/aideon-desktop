use aideon_mneme_store::api::{
    CreateScenarioInput, ExportOpsInput, ExportOptions, ListEntitiesInput, ReadEntityAtTimeInput,
    SnapshotOptions, TraverseAtTimeInput,
};
use aideon_mneme_store::ops::{
    CreateEdgeInput, CreateNodeInput, OpEnvelope, OpPayload, OpType, SetPropIntervalInput,
};
use aideon_mneme_store::schema::{FieldDef, MetamodelBatch, TypeDef, TypeFieldDef};
use aideon_mneme_store::value::{EntityKind, Layer, MergePolicy, Value, ValueType};
use aideon_mneme_store::{
    ActorId, ChangeFeedApi, CompareOp, Direction, FieldFilter, GraphReadApi, GraphWriteApi, Hlc,
    Id, MetamodelApi, MnemeConfig, MnemeExportApi, MnemeResult, MnemeSnapshotApi, MnemeStore,
    PartitionId, PropertyWriteApi, ScenarioApi, SyncApi, ValidTime,
};
use tempfile::tempdir;

#[tokio::test]
async fn store_roundtrip_graph_and_sync() -> MnemeResult<()> {
    let dir = tempdir().expect("tempdir");
    let base = dir.path();
    let config = MnemeConfig::default_sqlite(base.join("store.sqlite").to_string_lossy());
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
                edge_type_rules: vec![],
                metamodel_version: Some("v1".to_string()),
                metamodel_source: Some("tests".to_string()),
            },
        )
        .await?;

    let node_a = Id::new();
    let node_b = Id::new();
    store
        .create_node(CreateNodeInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            node_id: node_a,
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
            node_id: node_b,
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
            entity_id: node_a,
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
            entity_id: node_b,
            field_id,
            value: Value::Str("beta".to_string()),
            valid_from: ValidTime(0),
            valid_to: None,
            layer: Layer::Actual,
            write_options: None,
        })
        .await?;

    let read = store
        .read_entity_at_time(ReadEntityAtTimeInput {
            partition,
            scenario_id: None,
            security_context: None,
            entity_id: node_a,
            at_valid_time: ValidTime(0),
            as_of_asserted_at: None,
            field_ids: None,
            include_defaults: true,
        })
        .await?;
    assert_eq!(read.entity_id, node_a);
    assert!(read.properties.contains_key(&field_id));

    let filtered = store
        .list_entities(ListEntitiesInput {
            partition,
            scenario_id: None,
            security_context: None,
            kind: Some(EntityKind::Node),
            type_id: Some(type_id),
            at_valid_time: ValidTime(0),
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
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].entity_id, node_a);

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

    let traversed = store
        .traverse_at_time(TraverseAtTimeInput {
            partition,
            scenario_id: None,
            security_context: None,
            from_entity_id: node_a,
            direction: Direction::Out,
            edge_type_id: None,
            at_valid_time: ValidTime(0),
            as_of_asserted_at: None,
            limit: 10,
        })
        .await?;
    assert_eq!(traversed.len(), 1);

    let _ops = store
        .export_ops(ExportOpsInput {
            partition,
            scenario_id: None,
            since_asserted_at: None,
            limit: 100,
        })
        .await?;
    assert!(!_ops.is_empty());

    let other_partition = PartitionId(Id::new());
    let ingest_node = Id::new();
    let ingest_payload = OpPayload::CreateNode(CreateNodeInput {
        partition: other_partition,
        scenario_id: None,
        actor,
        asserted_at: Hlc::now(),
        node_id: ingest_node,
        type_id: None,
        acl_group_id: None,
        owner_actor_id: None,
        visibility: None,
        write_options: None,
    });
    let op_envelope = OpEnvelope {
        op_id: aideon_mneme_store::OpId(Id::new()),
        actor_id: actor,
        asserted_at: Hlc::now(),
        op_type: OpType::CreateNode as u16,
        payload: serde_json::to_vec(&ingest_payload).expect("payload"),
        deps: Vec::new(),
    };
    store.ingest_ops(other_partition, vec![op_envelope]).await?;
    let copied = store
        .read_entity_at_time(ReadEntityAtTimeInput {
            partition: other_partition,
            scenario_id: None,
            security_context: None,
            entity_id: ingest_node,
            at_valid_time: ValidTime(0),
            as_of_asserted_at: None,
            field_ids: None,
            include_defaults: true,
        })
        .await?;
    assert_eq!(copied.entity_id, ingest_node);

    let head = store.get_partition_head(partition).await?;
    assert!(head.as_i64() > 0);

    let changes = store.get_changes_since(partition, None, 10).await?;
    assert!(!changes.is_empty());

    let scenario_id = store
        .create_scenario(CreateScenarioInput {
            partition,
            actor,
            asserted_at: Hlc::now(),
            name: "Scenario A".to_string(),
        })
        .await?;
    store
        .delete_scenario(partition, actor, Hlc::now(), scenario_id)
        .await?;

    let export_records = store
        .export_ops_stream(ExportOptions {
            partition,
            scenario_id: None,
            since_asserted_at: None,
            until_asserted_at: None,
            include_schema: true,
            include_data_ops: true,
            include_scenarios: true,
        })
        .await?
        .collect::<Vec<_>>();
    assert!(export_records.len() > 1);

    let snapshot = store
        .export_snapshot_stream(SnapshotOptions {
            partition_id: partition,
            scenario_id: None,
            as_of_asserted_at: Hlc::now(),
            include_facts: true,
            include_entities: true,
        })
        .await?
        .collect::<Vec<_>>();
    assert!(!snapshot.is_empty());

    Ok(())
}
