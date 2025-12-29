//! Host-side Mneme commands bridging renderer IPC calls to the Mneme store.

use aideon_praxis_facade::mneme::{
    AnalyticsApi, AnalyticsResultsApi, ChangeEvent, ChangeFeedApi, ClearPropIntervalInput,
    CompareOp, CounterUpdateInput, CreateEdgeInput, CreateNodeInput, CreateScenarioInput, Direction,
    EdgeTypeRule, EntityKind, FieldFilter, ExportOpsInput, GetGraphDegreeStatsInput,
    GetGraphEdgeTypeCountsInput, GetProjectionEdgesInput, GraphDegreeStat, GraphEdgeTypeCount,
    GraphReadApi, GraphWriteApi, ListEntitiesInput, ListEntitiesResultItem, MetamodelApi,
    MetamodelBatch, MnemeError, OpEnvelope, OpId, OrSetUpdateInput, PageRankRunSpec, PartitionId,
    PropertyWriteApi, ReadEntityAtTimeInput, ReadEntityAtTimeResult, SchemaVersion,
    SetEdgeExistenceIntervalInput, SetOp, SetPropIntervalInput, SyncApi, ScenarioApi,
    TraverseAtTimeInput, TraverseEdgeItem, ValidTime, Value, ProjectionEdge, MnemeProcessingApi,
    TriggerProcessingInput, TriggerRetentionInput, TriggerCompactionInput, RetentionPolicy,
    RunWorkerInput, JobSummary, DiagnosticsApi, IntegrityHead, SchemaHead, SchemaManifest,
    ExplainResolutionInput, ExplainResolutionResult, ExplainTraversalInput, ExplainTraversalResult,
    ExportOptions, ExportRecord, ImportOptions, ImportReport, MnemeExportApi, MnemeImportApi,
    MnemeSnapshotApi, SnapshotOptions, ValidationRulesApi, ValidationRule, ComputedRulesApi,
    ComputedRule, ComputedCacheApi, ComputedCacheEntry, ListComputedCacheInput,
};
use aideon_praxis_facade::mneme::{ActorId, Hlc, Layer, ScenarioId};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::State;
use tauri::Window;
use tauri::async_runtime::spawn;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::sync::oneshot;

use crate::worker::WorkerState;

static SUBSCRIPTION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertMetamodelBatchInput {
    pub partition_id: PartitionId,
    pub actor_id: ActorId,
    pub asserted_at: String,
    pub batch: MetamodelBatch,
    #[serde(default)]
    pub scenario_id: Option<ScenarioId>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileEffectiveSchemaInput {
    pub partition_id: PartitionId,
    pub actor_id: ActorId,
    pub asserted_at: String,
    pub type_id: aideon_praxis_facade::mneme::Id,
    #[serde(default)]
    pub scenario_id: Option<ScenarioId>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpResult {
    pub op_id: OpId,
}

#[tauri::command]
pub async fn mneme_upsert_metamodel_batch(
    state: State<'_, WorkerState>,
    payload: UpsertMetamodelBatchInput,
) -> Result<OpResult, HostError> {
    info!("host: mneme_upsert_metamodel_batch received");
    debug!(
        "host: mneme_upsert_metamodel_batch partition={:?} scenario={:?}",
        payload.partition_id, payload.scenario_id
    );
    let store = state.mneme();
    let op_id = store
        .upsert_metamodel_batch(
            payload.partition_id,
            payload.actor_id,
            parse_hlc(&payload.asserted_at)?,
            payload.batch,
        )
        .await
        .map_err(host_error)?;
    Ok(OpResult { op_id })
}

#[tauri::command]
pub async fn mneme_compile_effective_schema(
    state: State<'_, WorkerState>,
    payload: CompileEffectiveSchemaInput,
) -> Result<SchemaVersion, HostError> {
    info!("host: mneme_compile_effective_schema received");
    debug!(
        "host: mneme_compile_effective_schema partition={:?} type_id={:?}",
        payload.partition_id, payload.type_id
    );
    let store = state.mneme();
    let result = store
        .compile_effective_schema(
            payload.partition_id,
            payload.actor_id,
            parse_hlc(&payload.asserted_at)?,
            payload.type_id,
        )
        .await
        .map_err(host_error)?;
    Ok(result)
}

#[tauri::command]
pub async fn mneme_create_node(
    state: State<'_, WorkerState>,
    payload: CreateNodePayload,
) -> Result<OpResult, HostError> {
    let store = state.mneme();
    let op_id = store
        .create_node(CreateNodeInput {
            partition: payload.partition_id,
            scenario_id: payload.scenario_id,
            actor: payload.actor_id,
            asserted_at: parse_hlc(&payload.asserted_at)?,
            node_id: payload.node_id,
            type_id: payload.type_id,
            acl_group_id: payload.acl_group_id,
            owner_actor_id: payload.owner_actor_id,
            visibility: payload.visibility,
            write_options: None,
        })
        .await
        .map_err(host_error)?;
    Ok(OpResult { op_id })
}

#[tauri::command]
pub async fn mneme_create_edge(
    state: State<'_, WorkerState>,
    payload: CreateEdgePayload,
) -> Result<OpResult, HostError> {
    let store = state.mneme();
    let op_id = store
        .create_edge(CreateEdgeInput {
            partition: payload.partition_id,
            scenario_id: payload.scenario_id,
            actor: payload.actor_id,
            asserted_at: parse_hlc(&payload.asserted_at)?,
            edge_id: payload.edge_id,
            type_id: payload.type_id,
            src_id: payload.src_id,
            dst_id: payload.dst_id,
            exists_valid_from: parse_valid_time(&payload.exists_valid_from)?,
            exists_valid_to: match payload.exists_valid_to {
                Some(value) => Some(parse_valid_time(&value)?),
                None => None,
            },
            layer: payload.layer.unwrap_or_else(Layer::default_actual),
            weight: payload.weight,
            acl_group_id: payload.acl_group_id,
            owner_actor_id: payload.owner_actor_id,
            visibility: payload.visibility,
            write_options: None,
        })
        .await
        .map_err(host_error)?;
    Ok(OpResult { op_id })
}

#[tauri::command]
pub async fn mneme_set_edge_existence_interval(
    state: State<'_, WorkerState>,
    payload: SetEdgeExistencePayload,
) -> Result<OpResult, HostError> {
    let store = state.mneme();
    let op_id = store
        .set_edge_existence_interval(SetEdgeExistenceIntervalInput {
            partition: payload.partition_id,
            scenario_id: payload.scenario_id,
            actor: payload.actor_id,
            asserted_at: parse_hlc(&payload.asserted_at)?,
            edge_id: payload.edge_id,
            valid_from: parse_valid_time(&payload.valid_from)?,
            valid_to: match payload.valid_to {
                Some(value) => Some(parse_valid_time(&value)?),
                None => None,
            },
            layer: payload.layer.unwrap_or_else(Layer::default_actual),
            is_tombstone: payload.is_tombstone.unwrap_or(false),
            write_options: None,
        })
        .await
        .map_err(host_error)?;
    Ok(OpResult { op_id })
}

#[tauri::command]
pub async fn mneme_tombstone_entity(
    state: State<'_, WorkerState>,
    payload: TombstoneEntityPayload,
) -> Result<OpResult, HostError> {
    let store = state.mneme();
    let op_id = store
        .tombstone_entity(
            payload.partition_id,
            payload.scenario_id,
            payload.actor_id,
            parse_hlc(&payload.asserted_at)?,
            payload.entity_id,
        )
        .await
        .map_err(host_error)?;
    Ok(OpResult { op_id })
}

#[tauri::command]
pub async fn mneme_set_property_interval(
    state: State<'_, WorkerState>,
    payload: SetPropertyIntervalPayload,
) -> Result<OpResult, HostError> {
    let store = state.mneme();
    let op_id = store
        .set_property_interval(SetPropIntervalInput {
            partition: payload.partition_id,
            scenario_id: payload.scenario_id,
            actor: payload.actor_id,
            asserted_at: parse_hlc(&payload.asserted_at)?,
            entity_id: payload.entity_id,
            field_id: payload.field_id,
            value: payload.value,
            valid_from: parse_valid_time(&payload.valid_from)?,
            valid_to: match payload.valid_to {
                Some(value) => Some(parse_valid_time(&value)?),
                None => None,
            },
            layer: payload.layer.unwrap_or_else(Layer::default_actual),
            write_options: None,
        })
        .await
        .map_err(host_error)?;
    Ok(OpResult { op_id })
}

#[tauri::command]
pub async fn mneme_clear_property_interval(
    state: State<'_, WorkerState>,
    payload: ClearPropertyIntervalPayload,
) -> Result<OpResult, HostError> {
    let store = state.mneme();
    let op_id = store
        .clear_property_interval(ClearPropIntervalInput {
            partition: payload.partition_id,
            scenario_id: payload.scenario_id,
            actor: payload.actor_id,
            asserted_at: parse_hlc(&payload.asserted_at)?,
            entity_id: payload.entity_id,
            field_id: payload.field_id,
            valid_from: parse_valid_time(&payload.valid_from)?,
            valid_to: match payload.valid_to {
                Some(value) => Some(parse_valid_time(&value)?),
                None => None,
            },
            layer: payload.layer.unwrap_or_else(Layer::default_actual),
            write_options: None,
        })
        .await
        .map_err(host_error)?;
    Ok(OpResult { op_id })
}

#[tauri::command]
pub async fn mneme_or_set_update(
    state: State<'_, WorkerState>,
    payload: OrSetUpdatePayload,
) -> Result<OpResult, HostError> {
    let store = state.mneme();
    let op_id = store
        .or_set_update(OrSetUpdateInput {
            partition: payload.partition_id,
            scenario_id: payload.scenario_id,
            actor: payload.actor_id,
            asserted_at: parse_hlc(&payload.asserted_at)?,
            entity_id: payload.entity_id,
            field_id: payload.field_id,
            op: payload.op,
            element: payload.element,
            valid_from: parse_valid_time(&payload.valid_from)?,
            valid_to: match payload.valid_to {
                Some(value) => Some(parse_valid_time(&value)?),
                None => None,
            },
            layer: payload.layer.unwrap_or_else(Layer::default_actual),
            write_options: None,
        })
        .await
        .map_err(host_error)?;
    Ok(OpResult { op_id })
}

#[tauri::command]
pub async fn mneme_counter_update(
    state: State<'_, WorkerState>,
    payload: CounterUpdatePayload,
) -> Result<OpResult, HostError> {
    let store = state.mneme();
    let op_id = store
        .counter_update(CounterUpdateInput {
            partition: payload.partition_id,
            scenario_id: payload.scenario_id,
            actor: payload.actor_id,
            asserted_at: parse_hlc(&payload.asserted_at)?,
            entity_id: payload.entity_id,
            field_id: payload.field_id,
            delta: payload.delta,
            valid_from: parse_valid_time(&payload.valid_from)?,
            valid_to: match payload.valid_to {
                Some(value) => Some(parse_valid_time(&value)?),
                None => None,
            },
            layer: payload.layer.unwrap_or_else(Layer::default_actual),
            write_options: None,
        })
        .await
        .map_err(host_error)?;
    Ok(OpResult { op_id })
}

#[tauri::command]
pub async fn mneme_read_entity_at_time(
    state: State<'_, WorkerState>,
    payload: ReadEntityAtTimePayload,
) -> Result<ReadEntityAtTimeResult, HostError> {
    let store = state.mneme();
    let as_of = payload
        .as_of_asserted_at
        .as_deref()
        .map(parse_hlc)
        .transpose()?;
    store
        .read_entity_at_time(ReadEntityAtTimeInput {
            partition: payload.partition_id,
            scenario_id: payload.scenario_id,
            security_context: None,
            entity_id: payload.entity_id,
            at_valid_time: parse_valid_time(&payload.at)?,
            as_of_asserted_at: as_of,
            field_ids: payload.field_ids,
            include_defaults: payload.include_defaults.unwrap_or(false),
        })
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_traverse_at_time(
    state: State<'_, WorkerState>,
    payload: TraverseAtTimePayload,
) -> Result<Vec<TraverseEdgeItem>, HostError> {
    let store = state.mneme();
    let as_of = payload
        .as_of_asserted_at
        .as_deref()
        .map(parse_hlc)
        .transpose()?;
    store
        .traverse_at_time(TraverseAtTimeInput {
            partition: payload.partition_id,
            scenario_id: payload.scenario_id,
            security_context: None,
            from_entity_id: payload.from_entity_id,
            direction: payload.direction,
            edge_type_id: payload.edge_type_id,
            at_valid_time: parse_valid_time(&payload.at)?,
            as_of_asserted_at: as_of,
            limit: payload.limit.unwrap_or(200),
        })
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_list_entities(
    state: State<'_, WorkerState>,
    payload: ListEntitiesPayload,
) -> Result<Vec<ListEntitiesResultItem>, HostError> {
    let store = state.mneme();
    let as_of = payload
        .as_of_asserted_at
        .as_deref()
        .map(parse_hlc)
        .transpose()?;
    let filters = payload
        .filters
        .unwrap_or_default()
        .into_iter()
        .map(|filter| FieldFilter {
            field_id: filter.field_id,
            op: filter.op,
            value: filter.value,
        })
        .collect();
    store
        .list_entities(ListEntitiesInput {
            partition: payload.partition_id,
            scenario_id: payload.scenario_id,
            security_context: None,
            kind: payload.kind,
            type_id: payload.type_id,
            at_valid_time: parse_valid_time(&payload.at)?,
            as_of_asserted_at: as_of,
            filters,
            limit: payload.limit.unwrap_or(200),
            cursor: payload.cursor,
        })
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_get_changes_since(
    state: State<'_, WorkerState>,
    payload: GetChangesSincePayload,
) -> Result<Vec<ChangeEvent>, HostError> {
    let store = state.mneme();
    store
        .get_changes_since(
            payload.partition_id,
            payload.from_sequence,
            payload.limit.unwrap_or(500),
        )
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_subscribe_partition(
    state: State<'_, WorkerState>,
    window: Window,
    payload: SubscribePartitionPayload,
) -> Result<SubscriptionResult, HostError> {
    let store = state.mneme();
    let mut receiver = store
        .subscribe_partition(payload.partition_id, payload.from_sequence)
        .await
        .map_err(host_error)?;
    let (cancel_tx, mut cancel_rx) = oneshot::channel();
    let subscription_id = next_subscription_id();
    state
        .register_subscription(subscription_id.clone(), cancel_tx)
        .await;
    let event_name = payload
        .event_name
        .unwrap_or_else(|| "mneme_change_event".to_string());
    let window = window.clone();
    spawn(async move {
        loop {
            tokio::select! {
                _ = &mut cancel_rx => break,
                evt = receiver.recv() => {
                    match evt {
                        Some(change) => {
                            let _ = window.emit(&event_name, change);
                        }
                        None => break,
                    }
                }
            }
        }
    });
    Ok(SubscriptionResult { subscription_id })
}

#[tauri::command]
pub async fn mneme_unsubscribe_partition(
    state: State<'_, WorkerState>,
    payload: UnsubscribePartitionPayload,
) -> Result<bool, HostError> {
    Ok(state.cancel_subscription(&payload.subscription_id).await)
}

#[tauri::command]
pub async fn mneme_get_projection_edges(
    state: State<'_, WorkerState>,
    payload: GetProjectionEdgesPayload,
) -> Result<Vec<ProjectionEdge>, HostError> {
    let store = state.mneme();
    let as_of = payload
        .as_of_asserted_at
        .as_deref()
        .map(parse_hlc)
        .transpose()?;
    let at_valid_time = match payload.at {
        Some(value) => Some(parse_valid_time(&value)?),
        None => None,
    };
    store
        .get_projection_edges(GetProjectionEdgesInput {
            partition: payload.partition_id,
            scenario_id: payload.scenario_id,
            security_context: None,
            at_valid_time,
            as_of_asserted_at: as_of,
            edge_type_filter: payload.edge_type_filter,
            limit: payload.limit,
        })
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_get_graph_degree_stats(
    state: State<'_, WorkerState>,
    payload: GetGraphDegreeStatsPayload,
) -> Result<Vec<GraphDegreeStat>, HostError> {
    let store = state.mneme();
    let as_of_valid_time = match payload.as_of_valid_time {
        Some(value) => Some(parse_valid_time(&value)?),
        None => None,
    };
    store
        .get_graph_degree_stats(GetGraphDegreeStatsInput {
            partition: payload.partition_id,
            scenario_id: payload.scenario_id,
            as_of_valid_time,
            entity_ids: payload.entity_ids,
            limit: payload.limit,
        })
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_get_graph_edge_type_counts(
    state: State<'_, WorkerState>,
    payload: GetGraphEdgeTypeCountsPayload,
) -> Result<Vec<GraphEdgeTypeCount>, HostError> {
    let store = state.mneme();
    store
        .get_graph_edge_type_counts(GetGraphEdgeTypeCountsInput {
            partition: payload.partition_id,
            scenario_id: payload.scenario_id,
            edge_type_ids: payload.edge_type_ids,
            limit: payload.limit,
        })
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_store_pagerank_scores(
    state: State<'_, WorkerState>,
    payload: StorePageRankScoresPayload,
) -> Result<PageRankRunResult, HostError> {
    let store = state.mneme();
    let as_of_valid_time = match payload.as_of_valid_time {
        Some(value) => Some(parse_valid_time(&value)?),
        None => None,
    };
    let as_of_asserted_at = payload
        .as_of_asserted_at
        .as_deref()
        .map(parse_hlc)
        .transpose()?;
    let run_id = store
        .store_pagerank_scores(
            payload.partition_id,
            payload.actor_id,
            parse_hlc(&payload.asserted_at)?,
            as_of_valid_time,
            as_of_asserted_at,
            PageRankRunSpec {
                damping: payload.params.damping,
                max_iters: payload.params.max_iters,
                tol: payload.params.tol,
                personalised_seed: payload
                    .params
                    .personalised_seed
                    .map(|entries| entries.into_iter().map(|seed| (seed.id, seed.weight)).collect()),
            },
            payload
                .scores
                .into_iter()
                .map(|score| (score.id, score.score))
                .collect(),
        )
        .await
        .map_err(host_error)?;
    Ok(PageRankRunResult { run_id })
}

#[tauri::command]
pub async fn mneme_get_pagerank_scores(
    state: State<'_, WorkerState>,
    payload: GetPageRankScoresPayload,
) -> Result<Vec<PageRankScoreItem>, HostError> {
    let store = state.mneme();
    let scores = store
        .get_pagerank_scores(payload.partition_id, payload.run_id, payload.top_n)
        .await
        .map_err(host_error)?;
    Ok(scores
        .into_iter()
        .map(|(id, score)| PageRankScoreItem { id, score })
        .collect())
}

#[tauri::command]
pub async fn mneme_export_ops(
    state: State<'_, WorkerState>,
    payload: ExportOpsPayload,
) -> Result<Vec<OpEnvelope>, HostError> {
    let store = state.mneme();
    let since = payload
        .since_asserted_at
        .as_deref()
        .map(parse_hlc)
        .transpose()?;
    store
        .export_ops(ExportOpsInput {
            partition: payload.partition_id,
            scenario_id: payload.scenario_id,
            since_asserted_at: since,
            limit: payload.limit.unwrap_or(500),
        })
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_ingest_ops(
    state: State<'_, WorkerState>,
    payload: IngestOpsPayload,
) -> Result<(), HostError> {
    let store = state.mneme();
    let ops: Vec<OpEnvelope> = payload
        .ops
        .into_iter()
        .map(|op| {
            Ok(OpEnvelope {
                op_id: op.op_id,
                actor_id: op.actor_id,
                asserted_at: parse_hlc(&op.asserted_at)?,
                op_type: op.op_type,
                payload: op.payload,
                deps: op.deps,
            })
        })
        .collect::<Result<Vec<_>, HostError>>()?;
    store
        .ingest_ops(payload.partition_id, ops)
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_get_partition_head(
    state: State<'_, WorkerState>,
    payload: PartitionHeadPayload,
) -> Result<PartitionHeadResult, HostError> {
    let store = state.mneme();
    let head = store
        .get_partition_head(payload.partition_id)
        .await
        .map_err(host_error)?;
    Ok(PartitionHeadResult {
        head: head.as_i64().to_string(),
    })
}

#[tauri::command]
pub async fn mneme_create_scenario(
    state: State<'_, WorkerState>,
    payload: CreateScenarioPayload,
) -> Result<ScenarioId, HostError> {
    let store = state.mneme();
    let asserted_at = parse_hlc(&payload.asserted_at)?;
    store
        .create_scenario(CreateScenarioInput {
            partition: payload.partition_id,
            actor: payload.actor_id,
            asserted_at,
            name: payload.name,
        })
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_delete_scenario(
    state: State<'_, WorkerState>,
    payload: DeleteScenarioPayload,
) -> Result<(), HostError> {
    let store = state.mneme();
    let asserted_at = parse_hlc(&payload.asserted_at)?;
    store
        .delete_scenario(
            payload.partition_id,
            payload.actor_id,
            asserted_at,
            payload.scenario_id,
        )
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_export_ops_stream(
    state: State<'_, WorkerState>,
    payload: ExportOpsStreamPayload,
) -> Result<Vec<ExportRecord>, HostError> {
    let store = state.mneme();
    let since_asserted_at = payload
        .since_asserted_at
        .as_deref()
        .map(parse_hlc)
        .transpose()?;
    let until_asserted_at = payload
        .until_asserted_at
        .as_deref()
        .map(parse_hlc)
        .transpose()?;
    let options = ExportOptions {
        partition: payload.partition_id,
        scenario_id: payload.scenario_id,
        since_asserted_at,
        until_asserted_at,
        include_schema: payload.include_schema.unwrap_or(true),
        include_data_ops: payload.include_data_ops.unwrap_or(true),
        include_scenarios: payload.include_scenarios.unwrap_or(true),
    };
    let records = store
        .export_ops_stream(options)
        .await
        .map_err(host_error)?;
    Ok(records.collect())
}

#[tauri::command]
pub async fn mneme_import_ops_stream(
    state: State<'_, WorkerState>,
    payload: ImportOpsStreamPayload,
) -> Result<ImportReport, HostError> {
    let store = state.mneme();
    let options = ImportOptions {
        target_partition: payload.target_partition,
        scenario_id: payload.scenario_id,
        allow_partition_create: payload.allow_partition_create.unwrap_or(false),
        remap_actor_ids: payload.remap_actor_ids.unwrap_or_default(),
        strict_schema: payload.strict_schema.unwrap_or(false),
    };
    store
        .import_ops_stream(options, payload.records.into_iter())
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_export_snapshot_stream(
    state: State<'_, WorkerState>,
    payload: ExportSnapshotPayload,
) -> Result<Vec<ExportRecord>, HostError> {
    let store = state.mneme();
    let options = SnapshotOptions {
        partition_id: payload.partition_id,
        scenario_id: payload.scenario_id,
        as_of_asserted_at: parse_hlc(&payload.as_of_asserted_at)?,
        include_facts: payload.include_facts.unwrap_or(true),
        include_entities: payload.include_entities.unwrap_or(true),
    };
    let records = store
        .export_snapshot_stream(options)
        .await
        .map_err(host_error)?;
    Ok(records.collect())
}

#[tauri::command]
pub async fn mneme_import_snapshot_stream(
    state: State<'_, WorkerState>,
    payload: ImportSnapshotPayload,
) -> Result<(), HostError> {
    let store = state.mneme();
    let options = ImportOptions {
        target_partition: payload.target_partition,
        scenario_id: payload.scenario_id,
        allow_partition_create: payload.allow_partition_create.unwrap_or(false),
        remap_actor_ids: payload.remap_actor_ids.unwrap_or_default(),
        strict_schema: payload.strict_schema.unwrap_or(false),
    };
    store
        .import_snapshot_stream(options, payload.records.into_iter())
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_upsert_validation_rules(
    state: State<'_, WorkerState>,
    payload: UpsertValidationRulesPayload,
) -> Result<(), HostError> {
    let store = state.mneme();
    let asserted_at = parse_hlc(&payload.asserted_at)?;
    store
        .upsert_validation_rules(
            payload.partition_id,
            payload.actor_id,
            asserted_at,
            payload.rules,
        )
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_list_validation_rules(
    state: State<'_, WorkerState>,
    payload: ListValidationRulesPayload,
) -> Result<Vec<ValidationRule>, HostError> {
    let store = state.mneme();
    store
        .list_validation_rules(payload.partition_id)
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_upsert_computed_rules(
    state: State<'_, WorkerState>,
    payload: UpsertComputedRulesPayload,
) -> Result<(), HostError> {
    let store = state.mneme();
    let asserted_at = parse_hlc(&payload.asserted_at)?;
    store
        .upsert_computed_rules(
            payload.partition_id,
            payload.actor_id,
            asserted_at,
            payload.rules,
        )
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_list_computed_rules(
    state: State<'_, WorkerState>,
    payload: ListComputedRulesPayload,
) -> Result<Vec<ComputedRule>, HostError> {
    let store = state.mneme();
    store
        .list_computed_rules(payload.partition_id)
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_upsert_computed_cache(
    state: State<'_, WorkerState>,
    payload: UpsertComputedCachePayload,
) -> Result<(), HostError> {
    let store = state.mneme();
    let entries = payload
        .entries
        .into_iter()
        .map(|entry| {
            let valid_from = parse_valid_time(&entry.valid_from)?.0;
            let valid_to = entry
                .valid_to
                .as_deref()
                .map(parse_valid_time)
                .transpose()?
                .map(|time| time.0);
            let computed_asserted_at = parse_hlc(&entry.computed_asserted_at)?;
            Ok(ComputedCacheEntry {
                entity_id: entry.entity_id,
                field_id: entry.field_id,
                valid_from,
                valid_to,
                value: entry.value,
                rule_version_hash: entry.rule_version_hash,
                computed_asserted_at,
            })
        })
        .collect::<Result<Vec<_>, HostError>>()?;
    store
        .upsert_computed_cache(payload.partition_id, entries)
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_list_computed_cache(
    state: State<'_, WorkerState>,
    payload: ListComputedCachePayload,
) -> Result<Vec<ComputedCacheEntry>, HostError> {
    let store = state.mneme();
    let at_valid_time = payload
        .at_valid_time
        .as_deref()
        .map(parse_valid_time)
        .transpose()?;
    let input = ListComputedCacheInput {
        partition: payload.partition_id,
        entity_id: payload.entity_id,
        field_id: payload.field_id,
        at_valid_time,
        limit: payload.limit.unwrap_or(100),
    };
    store.list_computed_cache(input).await.map_err(host_error)
}

#[tauri::command]
pub async fn mneme_trigger_rebuild_effective_schema(
    state: State<'_, WorkerState>,
    payload: TriggerProcessingPayload,
) -> Result<(), HostError> {
    let store = state.mneme();
    store
        .trigger_rebuild_effective_schema(TriggerProcessingInput {
            partition: payload.partition_id,
            scenario_id: payload.scenario_id,
            reason: payload.reason,
        })
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_trigger_refresh_integrity(
    state: State<'_, WorkerState>,
    payload: TriggerProcessingPayload,
) -> Result<(), HostError> {
    let store = state.mneme();
    store
        .trigger_refresh_integrity(TriggerProcessingInput {
            partition: payload.partition_id,
            scenario_id: payload.scenario_id,
            reason: payload.reason,
        })
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_trigger_refresh_analytics_projections(
    state: State<'_, WorkerState>,
    payload: TriggerProcessingPayload,
) -> Result<(), HostError> {
    let store = state.mneme();
    store
        .trigger_refresh_analytics_projections(TriggerProcessingInput {
            partition: payload.partition_id,
            scenario_id: payload.scenario_id,
            reason: payload.reason,
        })
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_trigger_retention(
    state: State<'_, WorkerState>,
    payload: TriggerRetentionPayload,
) -> Result<(), HostError> {
    let store = state.mneme();
    store
        .trigger_retention(TriggerRetentionInput {
            partition: payload.partition_id,
            scenario_id: payload.scenario_id,
            policy: RetentionPolicy {
                keep_ops_days: payload.policy.keep_ops_days,
                keep_facts_days: payload.policy.keep_facts_days,
                keep_failed_jobs_days: payload.policy.keep_failed_jobs_days,
                keep_pagerank_runs_days: payload.policy.keep_pagerank_runs_days,
            },
            reason: payload.reason,
        })
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_trigger_compaction(
    state: State<'_, WorkerState>,
    payload: TriggerCompactionPayload,
) -> Result<(), HostError> {
    let store = state.mneme();
    store
        .trigger_compaction(TriggerCompactionInput {
            partition: payload.partition_id,
            scenario_id: payload.scenario_id,
            reason: payload.reason,
        })
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_run_processing_worker(
    state: State<'_, WorkerState>,
    payload: RunWorkerPayload,
) -> Result<RunWorkerResult, HostError> {
    let store = state.mneme();
    let jobs = store
        .run_processing_worker(RunWorkerInput {
            max_jobs: payload.max_jobs,
            lease_millis: payload.lease_millis,
        })
        .await
        .map_err(host_error)?;
    Ok(RunWorkerResult {
        jobs_processed: jobs,
    })
}

#[tauri::command]
pub async fn mneme_list_jobs(
    state: State<'_, WorkerState>,
    payload: ListJobsPayload,
) -> Result<Vec<JobSummary>, HostError> {
    let store = state.mneme();
    store
        .list_jobs(payload.partition_id, payload.status, payload.limit)
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_get_integrity_head(
    state: State<'_, WorkerState>,
    payload: IntegrityHeadPayload,
) -> Result<Option<IntegrityHead>, HostError> {
    let store = state.mneme();
    store
        .get_integrity_head(payload.partition_id, payload.scenario_id)
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_get_last_schema_compile(
    state: State<'_, WorkerState>,
    payload: SchemaHeadPayload,
) -> Result<Option<SchemaHead>, HostError> {
    let store = state.mneme();
    store
        .get_last_schema_compile(payload.partition_id, payload.type_id)
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_list_failed_jobs(
    state: State<'_, WorkerState>,
    payload: ListFailedJobsPayload,
) -> Result<Vec<JobSummary>, HostError> {
    let store = state.mneme();
    store
        .list_failed_jobs(payload.partition_id, payload.limit)
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_get_schema_manifest(
    state: State<'_, WorkerState>,
) -> Result<SchemaManifest, HostError> {
    let store = state.mneme();
    store.get_schema_manifest().await.map_err(host_error)
}

#[tauri::command]
pub async fn mneme_explain_resolution(
    state: State<'_, WorkerState>,
    payload: ExplainResolutionPayload,
) -> Result<ExplainResolutionResult, HostError> {
    let store = state.mneme();
    let as_of = payload
        .as_of_asserted_at
        .as_deref()
        .map(parse_hlc)
        .transpose()?;
    store
        .explain_resolution(ExplainResolutionInput {
            partition: payload.partition_id,
            scenario_id: payload.scenario_id,
            security_context: None,
            entity_id: payload.entity_id,
            field_id: payload.field_id,
            at_valid_time: parse_valid_time(&payload.at)?,
            as_of_asserted_at: as_of,
        })
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_explain_traversal(
    state: State<'_, WorkerState>,
    payload: ExplainTraversalPayload,
) -> Result<ExplainTraversalResult, HostError> {
    let store = state.mneme();
    let as_of = payload
        .as_of_asserted_at
        .as_deref()
        .map(parse_hlc)
        .transpose()?;
    store
        .explain_traversal(ExplainTraversalInput {
            partition: payload.partition_id,
            scenario_id: payload.scenario_id,
            security_context: None,
            edge_id: payload.edge_id,
            at_valid_time: parse_valid_time(&payload.at)?,
            as_of_asserted_at: as_of,
        })
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_get_effective_schema(
    state: State<'_, WorkerState>,
    partition_id: PartitionId,
    type_id: aideon_praxis_facade::mneme::Id,
) -> Result<Option<aideon_praxis_facade::mneme::EffectiveSchema>, HostError> {
    let store = state.mneme();
    store
        .get_effective_schema(partition_id, type_id)
        .await
        .map_err(host_error)
}

#[tauri::command]
pub async fn mneme_list_edge_type_rules(
    state: State<'_, WorkerState>,
    partition_id: PartitionId,
    edge_type_id: Option<aideon_praxis_facade::mneme::Id>,
) -> Result<Vec<EdgeTypeRule>, HostError> {
    let store = state.mneme();
    store
        .list_edge_type_rules(partition_id, edge_type_id)
        .await
        .map_err(host_error)
}

#[derive(Debug, Serialize)]
pub struct HostError {
    code: &'static str,
    message: String,
}

fn host_error(err: MnemeError) -> HostError {
    let code = match err {
        MnemeError::Storage { .. } => "storage_error",
        MnemeError::NotFound { .. } => "not_found",
        MnemeError::Validation { .. } => "validation_error",
        MnemeError::Conflict { .. } => "conflict_error",
        MnemeError::Processing { .. } => "processing_error",
        MnemeError::Sync { .. } => "sync_error",
    };
    error!("host: mneme error code={} detail={err}", code);
    HostError {
        code,
        message: err.to_string(),
    }
}

fn parse_hlc(value: &str) -> Result<Hlc, HostError> {
    let parsed = value
        .parse::<i64>()
        .map_err(|_| HostError {
            code: "invalid_time",
            message: format!("invalid assertedAt HLC value: {value}"),
        })?;
    Ok(Hlc::from_i64(parsed))
}

fn parse_valid_time(value: &str) -> Result<ValidTime, HostError> {
    if let Ok(raw) = value.parse::<i64>() {
        return Ok(ValidTime(raw));
    }
    let parsed = OffsetDateTime::parse(value, &Rfc3339).map_err(|_| HostError {
        code: "invalid_time",
        message: format!("invalid valid time value: {value}"),
    })?;
    Ok(ValidTime(parsed.unix_timestamp_nanos() / 1_000))
}

fn next_subscription_id() -> String {
    let next = SUBSCRIPTION_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("mneme-sub-{next}")
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNodePayload {
    pub partition_id: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub actor_id: ActorId,
    pub asserted_at: String,
    pub node_id: aideon_praxis_facade::mneme::Id,
    pub type_id: Option<aideon_praxis_facade::mneme::Id>,
    pub acl_group_id: Option<String>,
    pub owner_actor_id: Option<ActorId>,
    pub visibility: Option<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEdgePayload {
    pub partition_id: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub actor_id: ActorId,
    pub asserted_at: String,
    pub edge_id: aideon_praxis_facade::mneme::Id,
    pub type_id: Option<aideon_praxis_facade::mneme::Id>,
    pub src_id: aideon_praxis_facade::mneme::Id,
    pub dst_id: aideon_praxis_facade::mneme::Id,
    pub exists_valid_from: String,
    pub exists_valid_to: Option<String>,
    pub layer: Option<Layer>,
    pub weight: Option<f64>,
    pub acl_group_id: Option<String>,
    pub owner_actor_id: Option<ActorId>,
    pub visibility: Option<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetEdgeExistencePayload {
    pub partition_id: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub actor_id: ActorId,
    pub asserted_at: String,
    pub edge_id: aideon_praxis_facade::mneme::Id,
    pub valid_from: String,
    pub valid_to: Option<String>,
    pub layer: Option<Layer>,
    pub is_tombstone: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TombstoneEntityPayload {
    pub partition_id: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub actor_id: ActorId,
    pub asserted_at: String,
    pub entity_id: aideon_praxis_facade::mneme::Id,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetPropertyIntervalPayload {
    pub partition_id: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub actor_id: ActorId,
    pub asserted_at: String,
    pub entity_id: aideon_praxis_facade::mneme::Id,
    pub field_id: aideon_praxis_facade::mneme::Id,
    pub value: Value,
    pub valid_from: String,
    pub valid_to: Option<String>,
    pub layer: Option<Layer>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearPropertyIntervalPayload {
    pub partition_id: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub actor_id: ActorId,
    pub asserted_at: String,
    pub entity_id: aideon_praxis_facade::mneme::Id,
    pub field_id: aideon_praxis_facade::mneme::Id,
    pub valid_from: String,
    pub valid_to: Option<String>,
    pub layer: Option<Layer>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrSetUpdatePayload {
    pub partition_id: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub actor_id: ActorId,
    pub asserted_at: String,
    pub entity_id: aideon_praxis_facade::mneme::Id,
    pub field_id: aideon_praxis_facade::mneme::Id,
    pub op: SetOp,
    pub element: Value,
    pub valid_from: String,
    pub valid_to: Option<String>,
    pub layer: Option<Layer>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CounterUpdatePayload {
    pub partition_id: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub actor_id: ActorId,
    pub asserted_at: String,
    pub entity_id: aideon_praxis_facade::mneme::Id,
    pub field_id: aideon_praxis_facade::mneme::Id,
    pub delta: i64,
    pub valid_from: String,
    pub valid_to: Option<String>,
    pub layer: Option<Layer>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadEntityAtTimePayload {
    pub partition_id: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub entity_id: aideon_praxis_facade::mneme::Id,
    pub at: String,
    pub as_of_asserted_at: Option<String>,
    pub field_ids: Option<Vec<aideon_praxis_facade::mneme::Id>>,
    pub include_defaults: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraverseAtTimePayload {
    pub partition_id: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub from_entity_id: aideon_praxis_facade::mneme::Id,
    pub direction: Direction,
    pub edge_type_id: Option<aideon_praxis_facade::mneme::Id>,
    pub at: String,
    pub as_of_asserted_at: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListEntitiesPayload {
    pub partition_id: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub kind: Option<EntityKind>,
    pub type_id: Option<aideon_praxis_facade::mneme::Id>,
    pub at: String,
    pub as_of_asserted_at: Option<String>,
    pub filters: Option<Vec<ListEntitiesFilterPayload>>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListEntitiesFilterPayload {
    pub field_id: aideon_praxis_facade::mneme::Id,
    pub op: CompareOp,
    pub value: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetChangesSincePayload {
    pub partition_id: PartitionId,
    pub from_sequence: Option<i64>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribePartitionPayload {
    pub partition_id: PartitionId,
    pub from_sequence: Option<i64>,
    pub event_name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionResult {
    pub subscription_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsubscribePartitionPayload {
    pub subscription_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetProjectionEdgesPayload {
    pub partition_id: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub at: Option<String>,
    pub as_of_asserted_at: Option<String>,
    pub edge_type_filter: Option<Vec<aideon_praxis_facade::mneme::Id>>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetGraphDegreeStatsPayload {
    pub partition_id: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub as_of_valid_time: Option<String>,
    pub entity_ids: Option<Vec<aideon_praxis_facade::mneme::Id>>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetGraphEdgeTypeCountsPayload {
    pub partition_id: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub edge_type_ids: Option<Vec<aideon_praxis_facade::mneme::Id>>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorePageRankScoresPayload {
    pub partition_id: PartitionId,
    pub actor_id: ActorId,
    pub asserted_at: String,
    pub as_of_valid_time: Option<String>,
    pub as_of_asserted_at: Option<String>,
    pub params: PageRankParamsPayload,
    pub scores: Vec<PageRankScorePayload>,
    pub scenario_id: Option<ScenarioId>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageRankParamsPayload {
    pub damping: f64,
    pub max_iters: u32,
    pub tol: f64,
    pub personalised_seed: Option<Vec<PageRankSeedPayload>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageRankSeedPayload {
    pub id: aideon_praxis_facade::mneme::Id,
    pub weight: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageRankScorePayload {
    pub id: aideon_praxis_facade::mneme::Id,
    pub score: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPageRankScoresPayload {
    pub partition_id: PartitionId,
    pub run_id: aideon_praxis_facade::mneme::Id,
    pub top_n: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageRankRunResult {
    pub run_id: aideon_praxis_facade::mneme::Id,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageRankScoreItem {
    pub id: aideon_praxis_facade::mneme::Id,
    pub score: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportOpsPayload {
    pub partition_id: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub since_asserted_at: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestOpsPayload {
    pub partition_id: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub ops: Vec<OpEnvelopePayload>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpEnvelopePayload {
    pub op_id: OpId,
    pub actor_id: ActorId,
    pub asserted_at: String,
    pub op_type: u16,
    pub payload: Vec<u8>,
    pub deps: Vec<OpId>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartitionHeadResult {
    pub head: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartitionHeadPayload {
    pub partition_id: PartitionId,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateScenarioPayload {
    pub partition_id: PartitionId,
    pub actor_id: ActorId,
    pub asserted_at: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteScenarioPayload {
    pub partition_id: PartitionId,
    pub actor_id: ActorId,
    pub asserted_at: String,
    pub scenario_id: ScenarioId,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportOpsStreamPayload {
    pub partition_id: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub since_asserted_at: Option<String>,
    pub until_asserted_at: Option<String>,
    pub include_schema: Option<bool>,
    pub include_data_ops: Option<bool>,
    pub include_scenarios: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOpsStreamPayload {
    pub target_partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub allow_partition_create: Option<bool>,
    pub remap_actor_ids: Option<HashMap<ActorId, ActorId>>,
    pub strict_schema: Option<bool>,
    pub records: Vec<ExportRecord>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSnapshotPayload {
    pub partition_id: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub as_of_asserted_at: String,
    pub include_facts: Option<bool>,
    pub include_entities: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSnapshotPayload {
    pub target_partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub allow_partition_create: Option<bool>,
    pub remap_actor_ids: Option<HashMap<ActorId, ActorId>>,
    pub strict_schema: Option<bool>,
    pub records: Vec<ExportRecord>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertValidationRulesPayload {
    pub partition_id: PartitionId,
    pub actor_id: ActorId,
    pub asserted_at: String,
    pub rules: Vec<ValidationRule>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListValidationRulesPayload {
    pub partition_id: PartitionId,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertComputedRulesPayload {
    pub partition_id: PartitionId,
    pub actor_id: ActorId,
    pub asserted_at: String,
    pub rules: Vec<ComputedRule>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListComputedRulesPayload {
    pub partition_id: PartitionId,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputedCacheEntryPayload {
    pub entity_id: aideon_praxis_facade::mneme::Id,
    pub field_id: aideon_praxis_facade::mneme::Id,
    pub valid_from: String,
    pub valid_to: Option<String>,
    pub value: Value,
    pub rule_version_hash: String,
    pub computed_asserted_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertComputedCachePayload {
    pub partition_id: PartitionId,
    pub entries: Vec<ComputedCacheEntryPayload>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListComputedCachePayload {
    pub partition_id: PartitionId,
    pub entity_id: Option<aideon_praxis_facade::mneme::Id>,
    pub field_id: aideon_praxis_facade::mneme::Id,
    pub at_valid_time: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerProcessingPayload {
    pub partition_id: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionPolicyPayload {
    pub keep_ops_days: Option<u32>,
    pub keep_facts_days: Option<u32>,
    pub keep_failed_jobs_days: Option<u32>,
    pub keep_pagerank_runs_days: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerRetentionPayload {
    pub partition_id: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub policy: RetentionPolicyPayload,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerCompactionPayload {
    pub partition_id: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunWorkerPayload {
    pub max_jobs: u32,
    pub lease_millis: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunWorkerResult {
    pub jobs_processed: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListJobsPayload {
    pub partition_id: PartitionId,
    pub status: Option<u8>,
    pub limit: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityHeadPayload {
    pub partition_id: PartitionId,
    pub scenario_id: Option<ScenarioId>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaHeadPayload {
    pub partition_id: PartitionId,
    pub type_id: aideon_praxis_facade::mneme::Id,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListFailedJobsPayload {
    pub partition_id: PartitionId,
    pub limit: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplainResolutionPayload {
    pub partition_id: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub entity_id: aideon_praxis_facade::mneme::Id,
    pub field_id: aideon_praxis_facade::mneme::Id,
    pub at: String,
    pub as_of_asserted_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplainTraversalPayload {
    pub partition_id: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub edge_id: aideon_praxis_facade::mneme::Id,
    pub at: String,
    pub as_of_asserted_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_error_maps_codes() {
        let err = MnemeError::validation("bad");
        let mapped = host_error(err);
        assert_eq!(mapped.code, "validation_error");
        assert!(mapped.message.contains("bad"));

        let err = MnemeError::storage("fail");
        let mapped = host_error(err);
        assert_eq!(mapped.code, "storage_error");
    }
}
