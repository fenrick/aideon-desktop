use aideon_mneme_store::ops::CreateNodeInput;
use aideon_mneme_store::{
    ActorId, ExportOpsInput, GraphWriteApi, Hlc, Id, MnemeConfig, MnemeProcessingApi, MnemeResult,
    MnemeStore, PartitionId, RetentionPolicy, RunWorkerInput, SyncApi, TriggerRetentionInput,
    load_schema_manifest,
};
use tempfile::tempdir;

const MICROS_PER_DAY: i64 = 86_400_000_000;

#[tokio::test]
async fn retention_removes_old_ops() -> MnemeResult<()> {
    let dir = tempdir().expect("tempdir");
    let base = dir.path();
    let config = MnemeConfig::default_sqlite(base.join("mneme.sqlite").to_string_lossy());
    let store = MnemeStore::connect(&config, base).await?;
    let partition = PartitionId(Id::new());
    let actor = ActorId(Id::new());
    let node_id = Id::new();
    let old_micros = Hlc::now().physical_micros() - (2 * MICROS_PER_DAY);
    let asserted_at = Hlc::from_physical_micros(old_micros);
    store
        .create_node(CreateNodeInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at,
            node_id,
            type_id: None,
            acl_group_id: None,
            owner_actor_id: None,
            visibility: None,
            write_options: None,
        })
        .await?;

    store
        .trigger_retention(TriggerRetentionInput {
            partition,
            scenario_id: None,
            policy: RetentionPolicy {
                keep_ops_days: Some(1),
                keep_facts_days: None,
                keep_failed_jobs_days: None,
                keep_pagerank_runs_days: None,
            },
            reason: "test-retention".to_string(),
        })
        .await?;

    let processed = store
        .run_processing_worker(RunWorkerInput {
            max_jobs: 10,
            lease_millis: 1_000,
        })
        .await?;
    assert_eq!(processed, 1);

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

#[test]
fn schema_manifest_loads() {
    let manifest = load_schema_manifest().expect("schema manifest");
    assert!(
        manifest
            .tables
            .iter()
            .any(|table| table.name == "aideon_ops")
    );
}
