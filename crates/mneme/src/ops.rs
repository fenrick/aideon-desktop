use serde::{Deserialize, Serialize};

use crate::{ActorId, Hlc, Id, Layer, OpId, PartitionId, ScenarioId, ValidTime, Value};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[repr(u16)]
pub enum OpType {
    CreateNode = 1,
    CreateEdge = 2,
    TombstoneEntity = 3,
    SetProperty = 4,
    ClearProperty = 5,
    OrSetUpdate = 6,
    CounterUpdate = 7,
    UpsertMetamodelBatch = 8,
    CreateScenario = 9,
    DeleteScenario = 10,
    SetEdgeExistenceInterval = 11,
}

/// Append-only operation envelope stored in the op log.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OpEnvelope {
    pub op_id: OpId,
    pub actor_id: ActorId,
    pub asserted_at: crate::Hlc,
    pub op_type: u16,
    pub payload: Vec<u8>,
    pub deps: Vec<OpId>,
}

/// Create a new node entity (facts are added separately).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateNodeInput {
    pub partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub actor: ActorId,
    pub asserted_at: Hlc,
    pub node_id: Id,
    pub type_id: Option<Id>,
}

/// Create a new edge entity with an existence interval.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateEdgeInput {
    pub partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub actor: ActorId,
    pub asserted_at: Hlc,
    pub edge_id: Id,
    pub type_id: Option<Id>,
    pub src_id: Id,
    pub dst_id: Id,
    pub exists_valid_from: ValidTime,
    pub exists_valid_to: Option<ValidTime>,
    pub layer: Layer,
    pub weight: Option<f64>,
}

/// Modify an edge existence interval without changing endpoints.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SetEdgeExistenceIntervalInput {
    pub partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub actor: ActorId,
    pub asserted_at: Hlc,
    pub edge_id: Id,
    pub valid_from: ValidTime,
    pub valid_to: Option<ValidTime>,
    pub layer: Layer,
    pub is_tombstone: bool,
}

/// Set a typed property value over a valid-time interval.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SetPropIntervalInput {
    pub partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub actor: ActorId,
    pub asserted_at: Hlc,
    pub entity_id: Id,
    pub field_id: Id,
    pub value: Value,
    pub valid_from: ValidTime,
    pub valid_to: Option<ValidTime>,
    pub layer: Layer,
}

/// Clear a typed property value over a valid-time interval.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClearPropIntervalInput {
    pub partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub actor: ActorId,
    pub asserted_at: Hlc,
    pub entity_id: Id,
    pub field_id: Id,
    pub valid_from: ValidTime,
    pub valid_to: Option<ValidTime>,
    pub layer: Layer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SetOp {
    Add,
    Remove,
}

/// Update an OR-Set field by adding or removing an element.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrSetUpdateInput {
    pub partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub actor: ActorId,
    pub asserted_at: Hlc,
    pub entity_id: Id,
    pub field_id: Id,
    pub op: SetOp,
    pub element: Value,
    pub valid_from: ValidTime,
    pub valid_to: Option<ValidTime>,
    pub layer: Layer,
}

/// Update a counter field by adding a delta over a valid-time interval.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CounterUpdateInput {
    pub partition: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub actor: ActorId,
    pub asserted_at: Hlc,
    pub entity_id: Id,
    pub field_id: Id,
    pub delta: i64,
    pub valid_from: ValidTime,
    pub valid_to: Option<ValidTime>,
    pub layer: Layer,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OpPayload {
    CreateNode(CreateNodeInput),
    CreateEdge(CreateEdgeInput),
    SetEdgeExistenceInterval(SetEdgeExistenceIntervalInput),
    TombstoneEntity {
        partition: PartitionId,
        scenario_id: Option<ScenarioId>,
        actor: ActorId,
        asserted_at: Hlc,
        entity_id: Id,
    },
    SetProperty(SetPropIntervalInput),
    ClearProperty(ClearPropIntervalInput),
    OrSetUpdate(OrSetUpdateInput),
    CounterUpdate(CounterUpdateInput),
    UpsertMetamodelBatch(crate::MetamodelBatch),
    CreateScenario {
        partition: PartitionId,
        scenario_id: ScenarioId,
        actor: ActorId,
        asserted_at: Hlc,
        name: String,
    },
    DeleteScenario {
        partition: PartitionId,
        scenario_id: ScenarioId,
        actor: ActorId,
        asserted_at: Hlc,
    },
}

impl OpPayload {
    pub fn op_type(&self) -> OpType {
        match self {
            OpPayload::CreateNode(_) => OpType::CreateNode,
            OpPayload::CreateEdge(_) => OpType::CreateEdge,
            OpPayload::SetEdgeExistenceInterval(_) => OpType::SetEdgeExistenceInterval,
            OpPayload::TombstoneEntity { .. } => OpType::TombstoneEntity,
            OpPayload::SetProperty(_) => OpType::SetProperty,
            OpPayload::ClearProperty(_) => OpType::ClearProperty,
            OpPayload::OrSetUpdate(_) => OpType::OrSetUpdate,
            OpPayload::CounterUpdate(_) => OpType::CounterUpdate,
            OpPayload::UpsertMetamodelBatch(_) => OpType::UpsertMetamodelBatch,
            OpPayload::CreateScenario { .. } => OpType::CreateScenario,
            OpPayload::DeleteScenario { .. } => OpType::DeleteScenario,
        }
    }
}
