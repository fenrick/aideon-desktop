use aideon_mneme::ops::CreateNodeInput;
use aideon_mneme::{
    ActorId, ExportOptions, GraphWriteApi, Hlc, Id, ImportOptions, MnemeConfig, MnemeExportApi,
    MnemeImportApi, MnemeStore, PartitionId,
};
use tempfile::tempdir;

#[tokio::test]
async fn export_import_roundtrip_ops() -> aideon_mneme::MnemeResult<()> {
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
        })
        .await?;
    let records = source
        .export_ops_stream(ExportOptions {
            partition,
            scenario_id: None,
            since_asserted_at: None,
            until_asserted_at: None,
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
