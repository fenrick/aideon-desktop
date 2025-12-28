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

pub trait MnemeStore:
    MetamodelApi
    + GraphWriteApi
    + PropertyWriteApi
    + GraphReadApi
    + AnalyticsApi
    + AnalyticsResultsApi
    + SyncApi
    + ScenarioApi
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
        + Send
        + Sync
{
}
