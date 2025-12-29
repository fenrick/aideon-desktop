//! Host-side Mneme commands bridging renderer IPC calls to the Mneme store.

use aideon_praxis_facade::mneme::{
    ClearPropIntervalInput, CounterUpdateInput, CreateEdgeInput, CreateNodeInput, EdgeTypeRule,
    GraphWriteApi, MetamodelApi, MetamodelBatch, MnemeError, OpId, OrSetUpdateInput, PartitionId,
    PropertyWriteApi, SchemaVersion, SetEdgeExistenceIntervalInput, SetOp, SetPropIntervalInput,
    ValidTime, Value,
};
use aideon_praxis_facade::mneme::{ActorId, Hlc, Layer, ScenarioId};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use tauri::State;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::worker::WorkerState;

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
