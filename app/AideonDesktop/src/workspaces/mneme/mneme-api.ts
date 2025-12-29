import { invoke } from '@tauri-apps/api/core';
import type {
  ActorId,
  AssertedTime,
  ClearPropertyIntervalInput,
  CreateEdgeInput,
  CreateNodeInput,
  CounterUpdateInput,
  EdgeTypeRuleDef,
  FieldId,
  FieldDef,
  FieldFilter,
  ListEntitiesInput,
  ListEntitiesResultItem,
  OrSetUpdateInput,
  ReadEntityAtTimeInput,
  ReadEntityAtTimeResult,
  ReadValue,
  SetPropertyIntervalInput,
  SetEdgeExistenceIntervalInput,
  TraverseAtTimeInput,
  TraverseEdgeItem,
  Value,
  MetamodelBatch,
  OpId,
  PartitionId,
  SchemaCompileResult,
  TombstoneEntityInput,
  TypeDef,
  TypeFieldDef,
  TypeId,
} from 'dtos';

import { isTauri } from './platform';

const COMMANDS = {
  upsertMetamodelBatch: 'mneme_upsert_metamodel_batch',
  compileEffectiveSchema: 'mneme_compile_effective_schema',
  getEffectiveSchema: 'mneme_get_effective_schema',
  listEdgeTypeRules: 'mneme_list_edge_type_rules',
  createNode: 'mneme_create_node',
  createEdge: 'mneme_create_edge',
  setEdgeExistenceInterval: 'mneme_set_edge_existence_interval',
  tombstoneEntity: 'mneme_tombstone_entity',
  setPropertyInterval: 'mneme_set_property_interval',
  clearPropertyInterval: 'mneme_clear_property_interval',
  orSetUpdate: 'mneme_or_set_update',
  counterUpdate: 'mneme_counter_update',
  readEntityAtTime: 'mneme_read_entity_at_time',
  traverseAtTime: 'mneme_traverse_at_time',
  listEntities: 'mneme_list_entities',
} as const;

function toInvokeArgs<T extends object>(value: T): Record<string, unknown> {
  return value as Record<string, unknown>;
}

export interface UpsertMetamodelBatchInput {
  partitionId: PartitionId;
  actorId: ActorId;
  assertedAt: AssertedTime;
  batch: MetamodelBatch;
  scenarioId?: string;
}

export interface CompileEffectiveSchemaInput {
  partitionId: PartitionId;
  actorId: ActorId;
  assertedAt: AssertedTime;
  typeId: TypeId;
  scenarioId?: string;
}

export interface MnemeOpResult {
  opId: OpId;
}

export interface EffectiveSchema {
  typeId: TypeId;
  appliesTo: 'Node' | 'Edge';
  fields: Array<{
    fieldId: FieldId;
    valueType: 'str' | 'i64' | 'f64' | 'bool' | 'time' | 'ref' | 'blob' | 'json';
    cardinality: 'single' | 'multi';
    mergePolicy: 'LWW' | 'MV' | 'OR_SET' | 'COUNTER' | 'TEXT';
    required: boolean;
    defaultValue?: unknown;
    indexed: boolean;
    disallowOverlap?: boolean;
  }>;
}

export async function upsertMetamodelBatch(input: UpsertMetamodelBatchInput): Promise<MnemeOpResult> {
  if (!isTauri()) {
    return { opId: 'mock-op' };
  }
  try {
    return await invoke<MnemeOpResult>(COMMANDS.upsertMetamodelBatch, {
      partitionId: input.partitionId,
      actorId: input.actorId,
      assertedAt: input.assertedAt,
      scenarioId: input.scenarioId,
      batch: toRustMetamodelBatch(input.batch),
    });
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.upsertMetamodelBatch}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function compileEffectiveSchema(
  input: CompileEffectiveSchemaInput,
): Promise<SchemaCompileResult> {
  if (!isTauri()) {
    return { schemaVersionHash: 'mock-schema-hash' };
  }
  try {
    return await invoke<SchemaCompileResult>(
      COMMANDS.compileEffectiveSchema,
      toInvokeArgs(input),
    );
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.compileEffectiveSchema}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function createNode(input: CreateNodeInput): Promise<MnemeOpResult> {
  if (!isTauri()) {
    return { opId: 'mock-op' };
  }
  try {
    return await invoke<MnemeOpResult>(COMMANDS.createNode, toInvokeArgs(input));
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.createNode}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function createEdge(input: CreateEdgeInput): Promise<MnemeOpResult> {
  if (!isTauri()) {
    return { opId: 'mock-op' };
  }
  try {
    return await invoke<MnemeOpResult>(COMMANDS.createEdge, toInvokeArgs(input));
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.createEdge}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function setEdgeExistenceInterval(
  input: SetEdgeExistenceIntervalInput,
): Promise<MnemeOpResult> {
  if (!isTauri()) {
    return { opId: 'mock-op' };
  }
  try {
    return await invoke<MnemeOpResult>(
      COMMANDS.setEdgeExistenceInterval,
      toInvokeArgs(input),
    );
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.setEdgeExistenceInterval}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function tombstoneEntity(input: TombstoneEntityInput): Promise<MnemeOpResult> {
  if (!isTauri()) {
    return { opId: 'mock-op' };
  }
  try {
    return await invoke<MnemeOpResult>(COMMANDS.tombstoneEntity, toInvokeArgs(input));
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.tombstoneEntity}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function setPropertyInterval(input: SetPropertyIntervalInput): Promise<MnemeOpResult> {
  if (!isTauri()) {
    return { opId: 'mock-op' };
  }
  try {
    return await invoke<MnemeOpResult>(COMMANDS.setPropertyInterval, {
      ...input,
      value: toRustValue(input.value),
    });
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.setPropertyInterval}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function clearPropertyInterval(
  input: ClearPropertyIntervalInput,
): Promise<MnemeOpResult> {
  if (!isTauri()) {
    return { opId: 'mock-op' };
  }
  try {
    return await invoke<MnemeOpResult>(
      COMMANDS.clearPropertyInterval,
      toInvokeArgs(input),
    );
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.clearPropertyInterval}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function orSetUpdate(input: OrSetUpdateInput): Promise<MnemeOpResult> {
  if (!isTauri()) {
    return { opId: 'mock-op' };
  }
  try {
    return await invoke<MnemeOpResult>(COMMANDS.orSetUpdate, {
      ...input,
      element: toRustValue(input.element),
    });
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.orSetUpdate}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function counterUpdate(input: CounterUpdateInput): Promise<MnemeOpResult> {
  if (!isTauri()) {
    return { opId: 'mock-op' };
  }
  try {
    return await invoke<MnemeOpResult>(COMMANDS.counterUpdate, toInvokeArgs(input));
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.counterUpdate}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function readEntityAtTime(
  input: ReadEntityAtTimeInput,
): Promise<ReadEntityAtTimeResult> {
  if (!isTauri()) {
    return {
      entityId: input.entityId,
      kind: 'Node',
      isDeleted: false,
      properties: {},
    };
  }
  try {
    const raw = await invoke<RustReadEntityAtTimeResult>(COMMANDS.readEntityAtTime, input);
    return fromRustReadEntityAtTime(raw);
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.readEntityAtTime}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function traverseAtTime(
  input: TraverseAtTimeInput,
): Promise<TraverseEdgeItem[]> {
  if (!isTauri()) {
    return [];
  }
  try {
    const raw = await invoke<RustTraverseEdgeItem[]>(COMMANDS.traverseAtTime, input);
    return raw.map((edge) => ({
      edgeId: edge.edge_id,
      srcId: edge.src_id,
      dstId: edge.dst_id,
      edgeTypeId: edge.type_id ?? undefined,
    }));
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.traverseAtTime}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function listEntities(
  input: ListEntitiesInput,
): Promise<ListEntitiesResultItem[]> {
  if (!isTauri()) {
    return [];
  }
  try {
    const raw = await invoke<RustListEntitiesResultItem[]>(COMMANDS.listEntities, {
      ...input,
      filters: (input.filters ?? []).map(toRustFieldFilter),
    });
    return raw.map((item) => ({
      entityId: item.entity_id,
      kind: item.kind,
      typeId: item.type_id ?? undefined,
    }));
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.listEntities}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function getEffectiveSchema(
  partitionId: PartitionId,
  typeId: TypeId,
): Promise<EffectiveSchema | null> {
  if (!isTauri()) {
    return null;
  }
  try {
    const raw = await invoke<RustEffectiveSchema | null>(COMMANDS.getEffectiveSchema, {
      partitionId,
      typeId,
    });
    return raw ? fromRustEffectiveSchema(raw) : null;
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.getEffectiveSchema}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function listEdgeTypeRules(
  partitionId: PartitionId,
  edgeTypeId?: TypeId,
): Promise<EdgeTypeRuleDef[]> {
  if (!isTauri()) {
    return [];
  }
  try {
    const raw = await invoke<RustEdgeTypeRule[]>(COMMANDS.listEdgeTypeRules, {
      partitionId,
      edgeTypeId,
    });
    return raw.map(fromRustEdgeTypeRule);
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.listEdgeTypeRules}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

type RustValueType = 'Str' | 'I64' | 'F64' | 'Bool' | 'Time' | 'Ref' | 'Blob' | 'Json';
type RustMergePolicy = 'Lww' | 'Mv' | 'OrSet' | 'Counter' | 'Text';

interface RustTypeDef {
  type_id: TypeId;
  applies_to: 'Node' | 'Edge';
  label: string;
  is_abstract: boolean;
  parent_type_id?: TypeId;
}

interface RustFieldDef {
  field_id: FieldId;
  label: string;
  value_type: RustValueType;
  cardinality_multi: boolean;
  merge_policy: RustMergePolicy;
  is_indexed: boolean;
}

interface RustTypeFieldDef {
  type_id: TypeId;
  field_id: FieldId;
  is_required: boolean;
  default_value?: unknown;
  override_default?: boolean;
  tighten_required?: boolean;
}

interface RustEdgeTypeRule {
  edge_type_id: TypeId;
  allowed_src_type_ids: TypeId[];
  allowed_dst_type_ids: TypeId[];
  semantic_direction?: string | null;
}

interface RustMetamodelBatch {
  types: RustTypeDef[];
  fields: RustFieldDef[];
  type_fields: RustTypeFieldDef[];
  edge_type_rules: RustEdgeTypeRule[];
  metamodel_version?: string | null;
  metamodel_source?: string | null;
}

interface RustEffectiveSchemaField {
  field_id: FieldId;
  value_type: RustValueType;
  cardinality_multi: boolean;
  merge_policy: RustMergePolicy;
  is_required: boolean;
  default_value?: unknown;
  is_indexed: boolean;
  disallow_overlap: boolean;
}

interface RustEffectiveSchema {
  type_id: TypeId;
  applies_to: 'Node' | 'Edge';
  fields: RustEffectiveSchemaField[];
}

interface RustReadEntityAtTimeResult {
  entity_id: TypeId;
  kind: 'Node' | 'Edge';
  type_id?: TypeId;
  is_deleted: boolean;
  properties: Record<FieldId, RustReadValue>;
}

interface RustTraverseEdgeItem {
  edge_id: TypeId;
  src_id: TypeId;
  dst_id: TypeId;
  type_id?: TypeId;
}

interface RustListEntitiesResultItem {
  entity_id: TypeId;
  kind: 'Node' | 'Edge';
  type_id?: TypeId;
}

type RustValue =
  | { Str: string }
  | { I64: number }
  | { F64: number }
  | { Bool: boolean }
  | { Time: number }
  | { Ref: string }
  | { Blob: Uint8Array }
  | { Json: unknown };

type RustReadValue =
  | { Single: RustValue }
  | { Multi: RustValue[] }
  | { MultiLimited: { values: RustValue[]; more_available: boolean } };

const VALUE_TYPE_MAP: Record<EffectiveSchema['fields'][number]['valueType'], RustValueType> = {
  str: 'Str',
  i64: 'I64',
  f64: 'F64',
  bool: 'Bool',
  time: 'Time',
  ref: 'Ref',
  blob: 'Blob',
  json: 'Json',
};

const MERGE_POLICY_MAP: Record<
  EffectiveSchema['fields'][number]['mergePolicy'],
  RustMergePolicy
> = {
  LWW: 'Lww',
  MV: 'Mv',
  OR_SET: 'OrSet',
  COUNTER: 'Counter',
  TEXT: 'Text',
};

function toRustValueType(valueType: RustValueType | EffectiveSchema['fields'][number]['valueType']) {
  return VALUE_TYPE_MAP[valueType as EffectiveSchema['fields'][number]['valueType']] ?? valueType;
}

function toRustMergePolicy(
  mergePolicy: RustMergePolicy | EffectiveSchema['fields'][number]['mergePolicy'],
) {
  return MERGE_POLICY_MAP[mergePolicy as EffectiveSchema['fields'][number]['mergePolicy']] ?? mergePolicy;
}

function toRustType(def: TypeDef): RustTypeDef {
  return {
    type_id: def.typeId,
    applies_to: def.appliesTo,
    label: def.label,
    is_abstract: def.isAbstract,
    parent_type_id: def.parentTypeId,
  };
}

function toRustField(def: FieldDef): RustFieldDef {
  return {
    field_id: def.fieldId,
    label: def.label,
    value_type: toRustValueType(def.valueType),
    cardinality_multi: def.cardinality === 'multi',
    merge_policy: toRustMergePolicy(def.mergePolicy),
    is_indexed: def.indexed,
  };
}

function toRustTypeField(def: TypeFieldDef): RustTypeFieldDef {
  return {
    type_id: def.typeId,
    field_id: def.fieldId,
    is_required: def.required,
    default_value: def.defaultValue,
    override_default: def.overrideDefault,
    tighten_required: def.tightenRequired,
  };
}

function toRustEdgeTypeRule(def: EdgeTypeRuleDef): RustEdgeTypeRule {
  return {
    edge_type_id: def.edgeTypeId,
    semantic_direction: def.semanticDirection,
    allowed_src_type_ids: def.allowedSrcTypeIds ?? [],
    allowed_dst_type_ids: def.allowedDstTypeIds ?? [],
  };
}

function toRustMetamodelBatch(batch: MetamodelBatch): RustMetamodelBatch {
  return {
    types: batch.types.map(toRustType),
    fields: batch.fields.map(toRustField),
    type_fields: batch.typeFields.map(toRustTypeField),
    edge_type_rules: (batch.edgeTypeRules ?? []).map(toRustEdgeTypeRule),
    metamodel_version: undefined,
    metamodel_source: undefined,
  };
}

function toRustFieldFilter(filter: FieldFilter) {
  return {
    fieldId: filter.fieldId,
    op: filter.op,
    value: toRustValue(filter.value),
  };
}

function toRustValue(value: Value): RustValue {
  switch (value.t) {
    case 'str':
      return { Str: value.v };
    case 'i64': {
      const num = Number(value.v);
      if (!Number.isSafeInteger(num)) {
        throw new Error('Value.i64 exceeds safe integer range for IPC transport.');
      }
      return { I64: num };
    }
    case 'f64':
      return { F64: value.v };
    case 'bool':
      return { Bool: value.v };
    case 'time':
      return { Time: toValidTimeMicros(value.v) };
    case 'ref':
      return { Ref: value.v };
    case 'blob':
      return { Blob: value.v };
    case 'json':
      return { Json: value.v };
    default: {
      const _exhaustive: never = value;
      throw new Error(`Unsupported value type: ${String(_exhaustive)}`);
    }
  }
}

function toValidTimeMicros(value: string): number {
  const parsed = Date.parse(value);
  if (Number.isNaN(parsed)) {
    throw new Error(`Invalid ISO-8601 timestamp: ${value}`);
  }
  return parsed * 1000;
}

function fromRustValue(value: RustValue): Value {
  if ('Str' in value) {
    return { t: 'str', v: value.Str };
  }
  if ('I64' in value) {
    return { t: 'i64', v: BigInt(value.I64) };
  }
  if ('F64' in value) {
    return { t: 'f64', v: value.F64 };
  }
  if ('Bool' in value) {
    return { t: 'bool', v: value.Bool };
  }
  if ('Time' in value) {
    return { t: 'time', v: new Date(value.Time / 1000).toISOString() };
  }
  if ('Ref' in value) {
    return { t: 'ref', v: value.Ref };
  }
  if ('Blob' in value) {
    const blob = value.Blob instanceof Uint8Array ? value.Blob : Uint8Array.from(value.Blob);
    return { t: 'blob', v: blob };
  }
  if ('Json' in value) {
    return { t: 'json', v: value.Json };
  }
  throw new Error('Unsupported Rust value variant');
}

function fromRustReadEntityAtTime(raw: RustReadEntityAtTimeResult): ReadEntityAtTimeResult {
  const properties: Record<FieldId, ReadValue> = {};
  Object.entries(raw.properties).forEach(([fieldId, value]) => {
    properties[fieldId] = toReadValue(value);
  });
  return {
    entityId: raw.entity_id,
    kind: raw.kind,
    typeId: raw.type_id ?? undefined,
    isDeleted: raw.is_deleted,
    properties,
  };
}

function toReadValue(value: RustReadValue): ReadValue {
  if ('Single' in value) {
    return { k: 'single', v: fromRustValue(value.Single) };
  }
  if ('Multi' in value) {
    return { k: 'multi', v: value.Multi.map(fromRustValue) };
  }
  return {
    k: 'multi_limited',
    v: {
      values: value.MultiLimited.values.map(fromRustValue),
      moreAvailable: value.MultiLimited.more_available,
    },
  };
}

const VALUE_TYPE_FROM_RUST: Record<RustValueType, EffectiveSchema['fields'][number]['valueType']> = {
  Str: 'str',
  I64: 'i64',
  F64: 'f64',
  Bool: 'bool',
  Time: 'time',
  Ref: 'ref',
  Blob: 'blob',
  Json: 'json',
};

const MERGE_POLICY_FROM_RUST: Record<RustMergePolicy, EffectiveSchema['fields'][number]['mergePolicy']> =
  {
    Lww: 'LWW',
    Mv: 'MV',
    OrSet: 'OR_SET',
    Counter: 'COUNTER',
    Text: 'TEXT',
  };

function fromRustEffectiveSchema(schema: RustEffectiveSchema): EffectiveSchema {
  return {
    typeId: schema.type_id,
    appliesTo: schema.applies_to,
    fields: schema.fields.map((field) => ({
      fieldId: field.field_id,
      valueType: VALUE_TYPE_FROM_RUST[field.value_type],
      cardinality: field.cardinality_multi ? 'multi' : 'single',
      mergePolicy: MERGE_POLICY_FROM_RUST[field.merge_policy],
      required: field.is_required,
      defaultValue: field.default_value,
      indexed: field.is_indexed,
      disallowOverlap: field.disallow_overlap,
    })),
  };
}

function fromRustEdgeTypeRule(rule: RustEdgeTypeRule): EdgeTypeRuleDef {
  return {
    edgeTypeId: rule.edge_type_id,
    semanticDirection: rule.semantic_direction ?? '',
    allowedSrcTypeIds: rule.allowed_src_type_ids,
    allowedDstTypeIds: rule.allowed_dst_type_ids,
  };
}
