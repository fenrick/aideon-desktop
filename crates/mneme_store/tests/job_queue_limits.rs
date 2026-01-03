use aideon_mneme_store::{
    Id, MnemeConfig, MnemeProcessingApi, MnemeStore, PartitionId, TriggerProcessingInput,
};
use tempfile::tempdir;

#[tokio::test]
async fn enforces_pending_job_limit() -> aideon_mneme_store::MnemeResult<()> {
    let dir = tempdir().expect("tempdir");
    let base = dir.path();
    let mut config = MnemeConfig::default_sqlite(base.join("mneme.sqlite").to_string_lossy());
    config.limits = Some(aideon_mneme_store::LimitsConfig {
        max_op_payload_bytes: None,
        max_blob_bytes: None,
        max_mv_values: None,
        max_pending_jobs: Some(1),
        max_ingest_batch: None,
    });
    let store = MnemeStore::connect(&config, base).await?;
    let partition = PartitionId(Id::new());

    store
        .trigger_rebuild_effective_schema(TriggerProcessingInput {
            partition,
            scenario_id: None,
            reason: "test-schema".to_string(),
        })
        .await?;

    let second = store
        .trigger_refresh_integrity(TriggerProcessingInput {
            partition,
            scenario_id: None,
            reason: "test-integrity".to_string(),
        })
        .await;

    assert!(second.is_err());
    Ok(())
}
