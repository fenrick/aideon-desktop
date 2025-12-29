use aideon_mneme_store::ops::CreateNodeInput;
use aideon_mneme_store::{
    ActorId, ExportOptions, GraphWriteApi, Hlc, Id, ImportOptions, MnemeConfig, MnemeExportApi,
    MnemeImportApi, MnemeSnapshotApi, MnemeStore, PartitionId, SnapshotOptions,
};
use tempfile::tempdir;

#[tokio::test]
async fn export_import_roundtrip_ops() -> aideon_mneme_store::MnemeResult<()> {
    let dir = tempdir().expect("tempdir");
    let base = dir.path();
    let config = MnemeConfig::default_sqlite(base.join("source.sqlite").to_string_lossy());
    let source = MnemeStore::connect(&config, base).await?;
    let partition = PartitionId(Id::new());
    let actor = ActorId(Id::new());
    source
        .create_node(CreateNodeInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            node_id: Id::new(),
            type_id: None,
            acl_group_id: None,
            owner_actor_id: None,
            visibility: None,
            write_options: None,
        })
        .await?;
    let records = source
        .export_ops_stream(ExportOptions {
            partition,
            scenario_id: None,
            since_asserted_at: None,
            until_asserted_at: None,
            include_schema: false,
            include_data_ops: true,
            include_scenarios: false,
        })
        .await?
        .collect::<Vec<_>>();

    let target_config = MnemeConfig::default_sqlite(base.join("target.sqlite").to_string_lossy());
    let target = MnemeStore::connect(&target_config, base).await?;
    let target_partition = PartitionId(Id::new());
    let report = target
        .import_ops_stream(
            ImportOptions {
                target_partition,
                scenario_id: None,
                allow_partition_create: true,
                remap_actor_ids: Default::default(),
                strict_schema: true,
            },
            records.into_iter(),
        )
        .await?;
    assert!(report.ops_imported > 0);
    Ok(())
}

#[tokio::test]
async fn snapshot_export_import_roundtrip() -> aideon_mneme_store::MnemeResult<()> {
    let dir = tempdir().expect("tempdir");
    let base = dir.path();
    let config = MnemeConfig::default_sqlite(base.join("source.sqlite").to_string_lossy());
    let source = MnemeStore::connect(&config, base).await?;
    let partition = PartitionId(Id::new());
    let actor = ActorId(Id::new());
    source
        .create_node(CreateNodeInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            node_id: Id::new(),
            type_id: None,
            acl_group_id: None,
            owner_actor_id: None,
            visibility: None,
            write_options: None,
        })
        .await?;

    let snapshot = source
        .export_snapshot_stream(SnapshotOptions {
            partition_id: partition,
            scenario_id: None,
            as_of_asserted_at: Hlc::now(),
            include_facts: true,
            include_entities: true,
        })
        .await?
        .collect::<Vec<_>>();

    let target_dir = tempdir().expect("tempdir");
    let target_base = target_dir.path();
    let target_config =
        MnemeConfig::default_sqlite(target_base.join("target.sqlite").to_string_lossy());
    let target = MnemeStore::connect(&target_config, target_base).await?;
    target
        .import_snapshot_stream(
            ImportOptions {
                target_partition: partition,
                scenario_id: None,
                allow_partition_create: true,
                remap_actor_ids: Default::default(),
                strict_schema: false,
            },
            snapshot.into_iter(),
        )
        .await?;

    let empty_dir = tempdir().expect("tempdir");
    let empty_base = empty_dir.path();
    let empty_config =
        MnemeConfig::default_sqlite(empty_base.join("empty.sqlite").to_string_lossy());
    let empty_store = MnemeStore::connect(&empty_config, empty_base).await?;
    let empty_partition = PartitionId(Id::new());
    let empty_result = empty_store
        .import_snapshot_stream(
            ImportOptions {
                target_partition: empty_partition,
                scenario_id: None,
                allow_partition_create: true,
                remap_actor_ids: Default::default(),
                strict_schema: false,
            },
            Vec::new().into_iter(),
        )
        .await;
    assert!(empty_result.is_err());

    Ok(())
}
