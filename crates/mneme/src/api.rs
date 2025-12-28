use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
    ActorId, ClearPropIntervalInput, CounterUpdateInput, CreateEdgeInput, CreateNodeInput,
    EdgeTypeRule, EffectiveSchema, EntityKind, Hlc, Id, MetamodelBatch, MnemeResult, OpEnvelope,
    OpId, OrSetUpdateInput, PartitionId, ScenarioId, SchemaVersion, SetPropIntervalInput,
    ValidTime, Value,
};

#[async_trait]
pub trait MetamodelApi {
    async fn upsert_metamodel_batch(
        &self,
        partition: PartitionId,
        actor: ActorId,
        asserted_at: Hlc,
        batch: MetamodelBatch,
    ) -> MnemeResult<()>;

    async fn compile_effective_schema(
        &self,
        partition: PartitionId,
        actor: ActorId,
        asserted_at: Hlc,
        type_id: Id,
    ) -> MnemeResult<SchemaVersion>;

    async fn get_effective_schema(
        &self,
        partition: PartitionId,
        type_id: Id,
    ) -> MnemeResult<Option<EffectiveSchema>>;

    async fn list_edge_type_rules(
        &self,
        partition: PartitionId,
        edge_type_id: Option<Id>,
    ) -> MnemeResult<Vec<EdgeTypeRule>>;
}

#[async_trait]
pub trait GraphWriteApi {
    async fn create_node(&self, input: CreateNodeInput) -> MnemeResult<OpId>;
    async fn create_edge(&self, input: CreateEdgeInput) -> MnemeResult<OpId>;
    async fn tombstone_entity(
        &self,
        partition: PartitionId,
        scenario_id: Option<ScenarioId>,
        actor: ActorId,
        asserted_at: Hlc,
        entity_id: Id,
    ) -> MnemeResult<OpId>;
}

#[async_trait]
pub trait PropertyWriteApi {
    async fn set_property_interval(&self, input: SetPropIntervalInput) -> MnemeResult<OpId>;
    async fn clear_property_interval(&self, input: ClearPropIntervalInput) -> MnemeResult<OpId>;
    async fn or_set_update(&self, input: OrSetUpdateInput) -> MnemeResult<OpId>;
    async fn counter_update(&self, input: CounterUpdateInput) -> MnemeResult<OpId>;
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReadValue {
    Single(Value),
    Multi(Vec<Value>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReadEntityAtTimeInput {
    pub partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub entity_id: Id,
    pub at_valid_time: ValidTime,
    pub as_of_asserted_at: Option<Hlc>,
    pub field_ids: Option<Vec<Id>>,
    pub include_defaults: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReadEntityAtTimeResult {
    pub entity_id: Id,
    pub kind: EntityKind,
    pub type_id: Option<Id>,
    pub is_deleted: bool,
    pub properties: HashMap<Id, ReadValue>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Out,
    In,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TraverseAtTimeInput {
    pub partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub from_entity_id: Id,
    pub direction: Direction,
    pub edge_type_id: Option<Id>,
    pub at_valid_time: ValidTime,
    pub as_of_asserted_at: Option<Hlc>,
    pub limit: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TraverseEdgeItem {
    pub edge_id: Id,
    pub src_id: Id,
    pub dst_id: Id,
    pub type_id: Option<Id>,
}

#[async_trait]
pub trait GraphReadApi {
    async fn read_entity_at_time(
        &self,
        input: ReadEntityAtTimeInput,
    ) -> MnemeResult<ReadEntityAtTimeResult>;
    async fn traverse_at_time(
        &self,
        input: TraverseAtTimeInput,
    ) -> MnemeResult<Vec<TraverseEdgeItem>>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProjectionEdge {
    pub src_id: Id,
    pub dst_id: Id,
    pub edge_type_id: Option<Id>,
    pub weight: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GetProjectionEdgesInput {
    pub partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub at_valid_time: Option<ValidTime>,
    pub as_of_asserted_at: Option<Hlc>,
    pub edge_type_filter: Option<Vec<Id>>,
}

#[async_trait]
pub trait AnalyticsApi {
    async fn get_projection_edges(
        &self,
        input: GetProjectionEdgesInput,
    ) -> MnemeResult<Vec<ProjectionEdge>>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PageRankRunSpec {
    pub damping: f64,
    pub max_iters: u32,
    pub tol: f64,
    pub personalised_seed: Option<Vec<(Id, f64)>>,
}

#[async_trait]
pub trait AnalyticsResultsApi {
    async fn store_pagerank_scores(
        &self,
        partition: PartitionId,
        actor: ActorId,
        as_of_valid_time: Option<ValidTime>,
        as_of_asserted_at: Option<Hlc>,
        spec: PageRankRunSpec,
        scores: Vec<(Id, f64)>,
    ) -> MnemeResult<Id>;

    async fn get_pagerank_scores(
        &self,
        partition: PartitionId,
        run_id: Id,
        top_n: u32,
    ) -> MnemeResult<Vec<(Id, f64)>>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExportOpsInput {
    pub partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub since_asserted_at: Option<Hlc>,
    pub limit: u32,
}

#[async_trait]
pub trait SyncApi {
    async fn export_ops(&self, input: ExportOpsInput) -> MnemeResult<Vec<OpEnvelope>>;
    async fn ingest_ops(&self, partition: PartitionId, ops: Vec<OpEnvelope>) -> MnemeResult<()>;
    async fn get_partition_head(&self, partition: PartitionId) -> MnemeResult<Hlc>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateScenarioInput {
    pub partition: PartitionId,
    pub actor: ActorId,
    pub asserted_at: Hlc,
    pub name: String,
}

#[async_trait]
pub trait ScenarioApi {
    async fn create_scenario(&self, input: CreateScenarioInput) -> MnemeResult<ScenarioId>;
    async fn delete_scenario(
        &self,
        partition: PartitionId,
        actor: ActorId,
        asserted_at: Hlc,
        scenario: ScenarioId,
    ) -> MnemeResult<()>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TriggerEvent {
    pub partition_id: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub op_id: OpId,
    pub op_type: u16,
    pub asserted_at: Hlc,
    pub entity_id: Option<Id>,
    pub type_id: Option<Id>,
    pub field_id: Option<Id>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JobSpec {
    pub job_type: String,
    pub priority: i32,
    pub payload: Vec<u8>,
    pub dedupe_key: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TriggerProcessingInput {
    pub partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunWorkerInput {
    pub max_jobs: u32,
    pub lease_millis: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JobSummary {
    pub partition: PartitionId,
    pub job_id: Id,
    pub job_type: String,
    pub status: u8,
    pub priority: i32,
    pub attempts: i32,
    pub max_attempts: i32,
    pub lease_expires_at: Option<i64>,
    pub created_asserted_at: Hlc,
    pub updated_asserted_at: Hlc,
    pub dedupe_key: Option<String>,
}

#[async_trait]
pub trait MnemeProcessingApi {
    async fn trigger_rebuild_effective_schema(
        &self,
        input: TriggerProcessingInput,
    ) -> MnemeResult<()>;
    async fn trigger_refresh_integrity(&self, input: TriggerProcessingInput) -> MnemeResult<()>;
    async fn trigger_refresh_analytics_projections(
        &self,
        input: TriggerProcessingInput,
    ) -> MnemeResult<()>;
    async fn run_processing_worker(&self, input: RunWorkerInput) -> MnemeResult<u32>;
    async fn list_jobs(
        &self,
        partition: PartitionId,
        status: Option<u8>,
        limit: u32,
    ) -> MnemeResult<Vec<JobSummary>>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChangeEvent {
    pub partition: PartitionId,
    pub sequence: i64,
    pub op_id: OpId,
    pub asserted_at: Hlc,
    pub entity_id: Option<Id>,
    pub change_kind: u8,
    pub payload: Option<serde_json::Value>,
}

#[async_trait]
pub trait ChangeFeedApi {
    async fn get_changes_since(
        &self,
        partition: PartitionId,
        from_sequence: Option<i64>,
        limit: u32,
    ) -> MnemeResult<Vec<ChangeEvent>>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ValidationRule {
    pub rule_id: Id,
    pub scope_kind: u8,
    pub scope_id: Option<Id>,
    pub severity: u8,
    pub template_kind: String,
    pub params: serde_json::Value,
}

#[async_trait]
pub trait ValidationRulesApi {
    async fn upsert_validation_rules(
        &self,
        partition: PartitionId,
        actor: ActorId,
        asserted_at: Hlc,
        rules: Vec<ValidationRule>,
    ) -> MnemeResult<()>;

    async fn list_validation_rules(
        &self,
        partition: PartitionId,
    ) -> MnemeResult<Vec<ValidationRule>>;
}

pub trait MnemeStore:
    MetamodelApi
    + GraphWriteApi
    + PropertyWriteApi
    + GraphReadApi
    + AnalyticsApi
    + AnalyticsResultsApi
    + SyncApi
    + ScenarioApi
    + MnemeProcessingApi
    + ChangeFeedApi
    + ValidationRulesApi
    + Send
    + Sync
{
}

impl<T> MnemeStore for T where
    T: MetamodelApi
        + GraphWriteApi
        + PropertyWriteApi
        + GraphReadApi
        + AnalyticsApi
        + AnalyticsResultsApi
        + SyncApi
        + ScenarioApi
        + MnemeProcessingApi
        + ChangeFeedApi
        + ValidationRulesApi
        + Send
        + Sync
{
}
