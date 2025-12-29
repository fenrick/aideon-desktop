use aideon_mneme_store::ops::{CreateEdgeInput, CreateNodeInput, SetPropIntervalInput};
use aideon_mneme_store::{
    ActorId, AnalyticsApi, DatabaseConfig, EntityKind, GetProjectionEdgesInput, GraphReadApi,
    GraphWriteApi, Hlc, Id, IntegrityConfig, Layer, LimitsConfig, ListEntitiesInput, MetamodelApi,
    MetamodelBatch, MnemeConfig, MnemeStore, PartitionId, PropertyWriteApi, ReadEntityAtTimeInput,
    TypeDef, TypeFieldDef, ValidTime, ValidationMode, Value, ValueType,
};
use tempfile::tempdir;

#[derive(Clone, Debug, PartialEq)]
struct ParitySnapshot {
    entities: Vec<aideon_mneme_store::ListEntitiesResultItem>,
    read: aideon_mneme_store::ReadEntityAtTimeResult,
    traverse: Vec<aideon_mneme_store::TraverseEdgeItem>,
    projection: Vec<aideon_mneme_store::ProjectionEdge>,
}

#[derive(Clone, Copy, Debug)]
struct ParityInput {
    partition: PartitionId,
    actor: ActorId,
    type_id: Id,
    field_id: Id,
    node_a: Id,
    node_b: Id,
    edge_id: Id,
}

fn remote_config(database: DatabaseConfig) -> MnemeConfig {
    MnemeConfig {
        database,
        pool: None,
        limits: Some(LimitsConfig::with_defaults()),
        integrity: Some(IntegrityConfig {
            record_overlap_warnings: Some(true),
        }),
        validation_mode: Some(ValidationMode::Error),
        failpoints: None,
    }
}

async fn populate_store(
    store: &MnemeStore,
    partition: PartitionId,
    actor: ActorId,
    type_id: Id,
    field_id: Id,
    node_a: Id,
    node_b: Id,
    edge_id: Id,
) -> aideon_mneme_store::MnemeResult<()> {
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
    Ok(())
}

async fn snapshot_store(
    store: &MnemeStore,
    partition: PartitionId,
    field_id: Id,
    node_a: Id,
    edge_id: Id,
) -> aideon_mneme_store::MnemeResult<ParitySnapshot> {
    let mut entities = store
        .list_entities(ListEntitiesInput {
            partition,
            scenario_id: None,
            security_context: None,
            kind: Some(EntityKind::Node),
            type_id: None,
            at_valid_time: ValidTime(1),
            as_of_asserted_at: None,
            filters: Vec::new(),
            limit: 100,
            cursor: None,
        })
        .await?;
    entities.sort_by(|a, b| a.entity_id.as_bytes().cmp(&b.entity_id.as_bytes()));

    let read = store
        .read_entity_at_time(ReadEntityAtTimeInput {
            partition,
            scenario_id: None,
            security_context: None,
            entity_id: node_a,
            at_valid_time: ValidTime(1),
            as_of_asserted_at: None,
            field_ids: Some(vec![field_id]),
            include_defaults: false,
        })
        .await?;

    let mut traverse = store
        .traverse_at_time(aideon_mneme_store::TraverseAtTimeInput {
            partition,
            scenario_id: None,
            security_context: None,
            from_entity_id: node_a,
            direction: aideon_mneme_store::Direction::Out,
            edge_type_id: None,
            at_valid_time: ValidTime(1),
            as_of_asserted_at: None,
            limit: 10,
        })
        .await?;
    traverse.sort_by(|a, b| a.edge_id.as_bytes().cmp(&b.edge_id.as_bytes()));

    let mut projection = store
        .get_projection_edges(GetProjectionEdgesInput {
            partition,
            scenario_id: None,
            security_context: None,
            at_valid_time: Some(ValidTime(1)),
            as_of_asserted_at: None,
            edge_type_filter: None,
            limit: None,
        })
        .await?;
    projection.sort_by(|a, b| a.edge_id.as_bytes().cmp(&b.edge_id.as_bytes()));

    let expected_edge_id = projection.first().map(|edge| edge.edge_id);
    assert_eq!(expected_edge_id, Some(edge_id));

    Ok(ParitySnapshot {
        entities,
        read,
        traverse,
        projection,
    })
}

async fn run_parity_test(
    config: MnemeConfig,
    input: ParityInput,
) -> aideon_mneme_store::MnemeResult<ParitySnapshot> {
    let dir = tempdir().expect("tempdir");
    let base = dir.path();
    let store = MnemeStore::connect(&config, base).await?;
    populate_store(
        &store,
        input.partition,
        input.actor,
        input.type_id,
        input.field_id,
        input.node_a,
        input.node_b,
        input.edge_id,
    )
    .await?;
    snapshot_store(
        &store,
        input.partition,
        input.field_id,
        input.node_a,
        input.edge_id,
    )
    .await
}

#[tokio::test]
async fn cross_db_parity_postgres() -> aideon_mneme_store::MnemeResult<()> {
    let url = match std::env::var("MNEME_PG_URL") {
        Ok(url) => url,
        Err(_) => return Ok(()),
    };
    let baseline_dir = tempdir().expect("tempdir");
    let baseline_config = MnemeConfig::default_sqlite(
        baseline_dir
            .path()
            .join("baseline.sqlite")
            .to_string_lossy(),
    );
    let input = ParityInput {
        partition: PartitionId(Id::new()),
        actor: ActorId(Id::new()),
        type_id: Id::new(),
        field_id: Id::new(),
        node_a: Id::new(),
        node_b: Id::new(),
        edge_id: Id::new(),
    };
    let baseline = run_parity_test(baseline_config, input).await?;
    let postgres = run_parity_test(remote_config(DatabaseConfig::Postgres { url }), input).await?;
    assert_eq!(baseline, postgres);
    Ok(())
}

#[tokio::test]
async fn cross_db_parity_mysql() -> aideon_mneme_store::MnemeResult<()> {
    let url = match std::env::var("MNEME_MYSQL_URL") {
        Ok(url) => url,
        Err(_) => return Ok(()),
    };
    let baseline_dir = tempdir().expect("tempdir");
    let baseline_config = MnemeConfig::default_sqlite(
        baseline_dir
            .path()
            .join("baseline.sqlite")
            .to_string_lossy(),
    );
    let input = ParityInput {
        partition: PartitionId(Id::new()),
        actor: ActorId(Id::new()),
        type_id: Id::new(),
        field_id: Id::new(),
        node_a: Id::new(),
        node_b: Id::new(),
        edge_id: Id::new(),
    };
    let baseline = run_parity_test(baseline_config, input).await?;
    let mysql = run_parity_test(remote_config(DatabaseConfig::Mysql { url }), input).await?;
    assert_eq!(baseline, mysql);
    Ok(())
}
