use aideon_mneme::ops::OpPayload;
use aideon_mneme::Id;
use aideon_mneme::{
    ActorId, CreateScenarioInput, MnemeConfig, MnemeStore, PartitionId, ScenarioApi, SyncApi,
};
use tempfile::tempdir;

#[tokio::test]
async fn create_scenario_emits_op_log_entry() -> aideon_mneme::MnemeResult<()> {
    let dir = tempdir().expect("tempdir");
    let base = dir.path();
    let config = MnemeConfig::default_sqlite(base.join("mneme.sqlite").to_string_lossy());
    let store = MnemeStore::connect(&config, base).await?;
    let partition = PartitionId(Id::new());
    let actor = ActorId(Id::new());
    let scenario = store
        .create_scenario(CreateScenarioInput {
            partition,
            actor,
            asserted_at: aideon_mneme::Hlc::now(),
            name: "plan-a".to_string(),
        })
        .await?;
    let ops = store
        .export_ops(aideon_mneme::ExportOpsInput {
            partition,
            scenario_id: Some(scenario),
            since_asserted_at: None,
            limit: 10,
        })
        .await?;
    let op = ops
        .into_iter()
        .find(|op| op.op_type == aideon_mneme::ops::OpType::CreateScenario as u16)
        .expect("create scenario op");
    let payload: OpPayload =
        serde_json::from_slice(&op.payload).expect("scenario op payload");
    match payload {
        OpPayload::CreateScenario { scenario_id, name, .. } => {
            assert_eq!(scenario_id, scenario);
            assert_eq!(name, "plan-a");
        }
        other => panic!("unexpected payload {other:?}"),
    }
    Ok(())
}
