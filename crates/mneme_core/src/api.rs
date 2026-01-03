use std::collections::HashMap;

use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};

use crate::{
    ActorId, ClearPropIntervalInput, CounterUpdateInput, CreateEdgeInput, CreateNodeInput,
    EdgeTypeRule, EffectiveSchema, EntityKind, Hlc, Id, MetamodelBatch, MnemeError, MnemeResult,
    OpEnvelope, OpId, OrSetUpdateInput, PartitionId, ScenarioId, SchemaManifest, SchemaVersion,
    SetEdgeExistenceIntervalInput, SetPropIntervalInput, ValidTime, Value,
};

#[async_trait]
pub trait MetamodelApi {
    async fn upsert_metamodel_batch(
        &self,
        partition: PartitionId,
        actor: ActorId,
        asserted_at: Hlc,
        batch: MetamodelBatch,
    ) -> MnemeResult<OpId>;

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
    async fn set_edge_existence_interval(
        &self,
        input: SetEdgeExistenceIntervalInput,
    ) -> MnemeResult<OpId>;
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ReadValue {
    Single(Value),
    Multi(Vec<Value>),
    MultiLimited {
        values: Vec<Value>,
        more_available: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReadEntityAtTimeInput {
    pub partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub security_context: Option<SecurityContext>,
    pub entity_id: Id,
    pub at_valid_time: ValidTime,
    pub as_of_asserted_at: Option<Hlc>,
    pub field_ids: Option<Vec<Id>>,
    pub include_defaults: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReadEntityAtTimeResult {
    pub entity_id: Id,
    pub kind: EntityKind,
    pub type_id: Option<Id>,
    pub is_deleted: bool,
    pub properties: HashMap<Id, ReadValue>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    Out,
    In,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TraverseAtTimeInput {
    pub partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub security_context: Option<SecurityContext>,
    pub from_entity_id: Id,
    pub direction: Direction,
    pub edge_type_id: Option<Id>,
    pub at_valid_time: ValidTime,
    pub as_of_asserted_at: Option<Hlc>,
    pub limit: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraverseEdgeItem {
    pub edge_id: Id,
    pub src_id: Id,
    pub dst_id: Id,
    pub type_id: Option<Id>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompareOp {
    Eq,
    Ne,
    Lt,
    Lte,
    Gt,
    Gte,
    Prefix,
    Contains,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldFilter {
    pub field_id: Id,
    pub op: CompareOp,
    pub value: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ListEntitiesInput {
    pub partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub security_context: Option<SecurityContext>,
    pub kind: Option<EntityKind>,
    pub type_id: Option<Id>,
    pub at_valid_time: ValidTime,
    pub as_of_asserted_at: Option<Hlc>,
    pub filters: Vec<FieldFilter>,
    pub limit: u32,
    pub cursor: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ListEntitiesResultItem {
    pub entity_id: Id,
    pub kind: EntityKind,
    pub type_id: Option<Id>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EntityCursor {
    pub entity_id: String,
}

pub fn encode_entity_cursor(entity_id: Id) -> MnemeResult<String> {
    let cursor = EntityCursor {
        entity_id: entity_id.to_uuid_string(),
    };
    let payload =
        serde_json::to_vec(&cursor).map_err(|err| MnemeError::storage(err.to_string()))?;
    Ok(URL_SAFE_NO_PAD.encode(payload))
}

pub fn decode_entity_cursor(cursor: &str) -> MnemeResult<Id> {
    let decoded = URL_SAFE_NO_PAD
        .decode(cursor.as_bytes())
        .map_err(|_| MnemeError::invalid("invalid cursor"))?;
    let parsed: EntityCursor =
        serde_json::from_slice(&decoded).map_err(|_| MnemeError::invalid("invalid cursor"))?;
    Id::from_uuid_str(&parsed.entity_id)
        .or_else(|_| Id::from_ulid_str(&parsed.entity_id))
        .map_err(|_| MnemeError::invalid("invalid cursor"))
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
    async fn list_entities(
        &self,
        input: ListEntitiesInput,
    ) -> MnemeResult<Vec<ListEntitiesResultItem>>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectionEdge {
    pub edge_id: Id,
    pub src_id: Id,
    pub dst_id: Id,
    pub edge_type_id: Option<Id>,
    pub weight: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GetProjectionEdgesInput {
    pub partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub security_context: Option<SecurityContext>,
    pub at_valid_time: Option<ValidTime>,
    pub as_of_asserted_at: Option<Hlc>,
    pub edge_type_filter: Option<Vec<Id>>,
    pub limit: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphDegreeStat {
    pub entity_id: Id,
    pub out_degree: i32,
    pub in_degree: i32,
    pub as_of_valid_time: Option<ValidTime>,
    pub computed_asserted_at: Hlc,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GetGraphDegreeStatsInput {
    pub partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub as_of_valid_time: Option<ValidTime>,
    pub entity_ids: Option<Vec<Id>>,
    pub limit: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphEdgeTypeCount {
    pub edge_type_id: Option<Id>,
    pub count: i32,
    pub computed_asserted_at: Hlc,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GetGraphEdgeTypeCountsInput {
    pub partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub edge_type_ids: Option<Vec<Id>>,
    pub limit: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SecurityContext {
    pub allowed_acl_groups: Option<Vec<String>>,
    pub allowed_owner_ids: Option<Vec<ActorId>>,
    pub min_visibility: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub keep_ops_days: Option<u32>,
    pub keep_facts_days: Option<u32>,
    pub keep_failed_jobs_days: Option<u32>,
    pub keep_pagerank_runs_days: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TriggerRetentionInput {
    pub partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub policy: RetentionPolicy,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TriggerCompactionInput {
    pub partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub reason: String,
}

#[async_trait]
pub trait AnalyticsApi {
    async fn get_projection_edges(
        &self,
        input: GetProjectionEdgesInput,
    ) -> MnemeResult<Vec<ProjectionEdge>>;
    async fn get_graph_degree_stats(
        &self,
        input: GetGraphDegreeStatsInput,
    ) -> MnemeResult<Vec<GraphDegreeStat>>;
    async fn get_graph_edge_type_counts(
        &self,
        input: GetGraphEdgeTypeCountsInput,
    ) -> MnemeResult<Vec<GraphEdgeTypeCount>>;
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
    pub next_run_after: Option<i64>,
    pub created_asserted_at: Hlc,
    pub updated_asserted_at: Hlc,
    pub dedupe_key: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JobRecord {
    pub partition: PartitionId,
    pub job_id: Id,
    pub job_type: String,
    pub status: u8,
    pub priority: i32,
    pub attempts: i32,
    pub max_attempts: i32,
    pub lease_expires_at: Option<i64>,
    pub next_run_after: Option<i64>,
    pub created_asserted_at: Hlc,
    pub updated_asserted_at: Hlc,
    pub dedupe_key: Option<String>,
    pub last_error: Option<String>,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JobContext {
    pub partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
}

#[async_trait]
pub trait Processor {
    fn name(&self) -> &'static str;
    fn interested_in(&self, op_type: u16) -> bool;
    fn plan_jobs(&self, evt: &TriggerEvent) -> Vec<JobSpec>;
    async fn run_job(&self, ctx: &JobContext, job: &JobRecord) -> MnemeResult<()>;
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
    async fn trigger_retention(&self, input: TriggerRetentionInput) -> MnemeResult<()>;
    async fn trigger_compaction(&self, input: TriggerCompactionInput) -> MnemeResult<()>;
    async fn run_processing_worker(&self, input: RunWorkerInput) -> MnemeResult<u32>;
    async fn list_jobs(
        &self,
        partition: PartitionId,
        status: Option<u8>,
        limit: u32,
    ) -> MnemeResult<Vec<JobSummary>>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IntegrityHead {
    pub partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub run_id: Id,
    pub updated_asserted_at: Hlc,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SchemaHead {
    pub partition: PartitionId,
    pub type_id: Id,
    pub schema_version_hash: String,
    pub updated_asserted_at: Hlc,
}

#[async_trait]
pub trait DiagnosticsApi {
    async fn get_integrity_head(
        &self,
        partition: PartitionId,
        scenario_id: Option<ScenarioId>,
    ) -> MnemeResult<Option<IntegrityHead>>;
    async fn get_last_schema_compile(
        &self,
        partition: PartitionId,
        type_id: Id,
    ) -> MnemeResult<Option<SchemaHead>>;
    async fn list_failed_jobs(
        &self,
        partition: PartitionId,
        limit: u32,
    ) -> MnemeResult<Vec<JobSummary>>;
    async fn get_schema_manifest(&self) -> MnemeResult<SchemaManifest>;
    async fn explain_resolution(
        &self,
        input: ExplainResolutionInput,
    ) -> MnemeResult<ExplainResolutionResult>;
    async fn explain_traversal(
        &self,
        input: ExplainTraversalInput,
    ) -> MnemeResult<ExplainTraversalResult>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExportOptions {
    pub partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub since_asserted_at: Option<Hlc>,
    pub until_asserted_at: Option<Hlc>,
    pub include_schema: bool,
    pub include_data_ops: bool,
    pub include_scenarios: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExportRecord {
    pub record_type: String,
    pub data: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImportOptions {
    pub target_partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub allow_partition_create: bool,
    pub remap_actor_ids: HashMap<ActorId, ActorId>,
    pub strict_schema: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImportReport {
    pub ops_imported: u32,
    pub ops_skipped: u32,
    pub errors: u32,
}

#[async_trait]
pub trait MnemeExportApi {
    async fn export_ops_stream(
        &self,
        options: ExportOptions,
    ) -> MnemeResult<Box<dyn Iterator<Item = ExportRecord> + Send>>;
}

#[async_trait]
pub trait MnemeImportApi {
    async fn import_ops_stream<I>(
        &self,
        options: ImportOptions,
        records: I,
    ) -> MnemeResult<ImportReport>
    where
        I: Iterator<Item = ExportRecord> + Send;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapshotOptions {
    pub partition_id: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub as_of_asserted_at: Hlc,
    pub include_facts: bool,
    pub include_entities: bool,
}

#[async_trait]
pub trait MnemeSnapshotApi {
    async fn export_snapshot_stream(
        &self,
        opts: SnapshotOptions,
    ) -> MnemeResult<Box<dyn Iterator<Item = ExportRecord> + Send>>;
    async fn import_snapshot_stream<I>(&self, opts: ImportOptions, records: I) -> MnemeResult<()>
    where
        I: Iterator<Item = ExportRecord> + Send;
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
    async fn subscribe_partition(
        &self,
        partition: PartitionId,
        from_sequence: Option<i64>,
    ) -> MnemeResult<tokio::sync::mpsc::Receiver<ChangeEvent>>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExplainResolutionInput {
    pub partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub security_context: Option<SecurityContext>,
    pub entity_id: Id,
    pub field_id: Id,
    pub at_valid_time: ValidTime,
    pub as_of_asserted_at: Option<Hlc>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExplainTraversalInput {
    pub partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub security_context: Option<SecurityContext>,
    pub edge_id: Id,
    pub at_valid_time: ValidTime,
    pub as_of_asserted_at: Option<Hlc>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExplainPrecedence {
    pub layer: i64,
    pub interval_width: i64,
    pub asserted_at: Hlc,
    pub op_id: Id,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExplainPropertyFact {
    pub value: Value,
    pub valid_from: i64,
    pub valid_to: Option<i64>,
    pub layer: i64,
    pub asserted_at: Hlc,
    pub op_id: Id,
    pub is_tombstone: bool,
    pub precedence: ExplainPrecedence,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExplainEdgeFact {
    pub edge_id: Id,
    pub src_id: Id,
    pub dst_id: Id,
    pub edge_type_id: Option<Id>,
    pub valid_from: i64,
    pub valid_to: Option<i64>,
    pub layer: i64,
    pub asserted_at: Hlc,
    pub op_id: Id,
    pub is_tombstone: bool,
    pub precedence: ExplainPrecedence,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExplainResolutionResult {
    pub entity_id: Id,
    pub field_id: Id,
    pub resolved: Option<ReadValue>,
    pub winner: Option<ExplainPropertyFact>,
    pub candidates: Vec<ExplainPropertyFact>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExplainTraversalResult {
    pub edge_id: Id,
    pub active: bool,
    pub winner: Option<ExplainEdgeFact>,
    pub candidates: Vec<ExplainEdgeFact>,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComputedRule {
    pub rule_id: Id,
    pub target_type_id: Option<Id>,
    pub output_field_id: Option<Id>,
    pub template_kind: String,
    pub params: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComputedCacheEntry {
    pub entity_id: Id,
    pub field_id: Id,
    pub valid_from: i64,
    pub valid_to: Option<i64>,
    pub value: Value,
    pub rule_version_hash: String,
    pub computed_asserted_at: Hlc,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ListComputedCacheInput {
    pub partition: PartitionId,
    pub entity_id: Option<Id>,
    pub field_id: Id,
    pub at_valid_time: Option<ValidTime>,
    pub limit: u32,
}

#[async_trait]
pub trait ComputedRulesApi {
    async fn upsert_computed_rules(
        &self,
        partition: PartitionId,
        actor: ActorId,
        asserted_at: Hlc,
        rules: Vec<ComputedRule>,
    ) -> MnemeResult<()>;

    async fn list_computed_rules(&self, partition: PartitionId) -> MnemeResult<Vec<ComputedRule>>;
}

#[async_trait]
pub trait ComputedCacheApi {
    async fn upsert_computed_cache(
        &self,
        partition: PartitionId,
        entries: Vec<ComputedCacheEntry>,
    ) -> MnemeResult<()>;

    async fn list_computed_cache(
        &self,
        input: ListComputedCacheInput,
    ) -> MnemeResult<Vec<ComputedCacheEntry>>;
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
    + ComputedRulesApi
    + ComputedCacheApi
    + DiagnosticsApi
    + MnemeExportApi
    + MnemeImportApi
    + MnemeSnapshotApi
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
        + ComputedRulesApi
        + ComputedCacheApi
        + DiagnosticsApi
        + MnemeExportApi
        + MnemeImportApi
        + MnemeSnapshotApi
        + Send
        + Sync
{
}
