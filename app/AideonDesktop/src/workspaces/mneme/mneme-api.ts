import { invoke } from '@tauri-apps/api/core';
import type {
  AssertedTime,
  ClearPropertyIntervalInput,
  CounterUpdateInput,
  CreateEdgeInput,
  CreateNodeInput,
  EdgeTypeRuleDefinition,
  FieldDefinition,
  FieldFilter,
  GetGraphDegreeStatsInput,
  GetGraphEdgeTypeCountsInput,
  GetPageRankScoresInput,
  GetProjectionEdgesInput,
  GraphDegreeStat,
  GraphEdgeTypeCount,
  ListEntitiesInput,
  ListEntitiesResultItem,
  MetamodelBatch,
  OrSetUpdateInput,
  PageRankRunParams,
  PageRankRunResult,
  PageRankScore,
  PageRankSeed,
  ProjectionEdge,
  ReadEntityAtTimeInput,
  ReadEntityAtTimeResult,
  ReadValue,
  SchemaCompileResult,
  SetEdgeExistenceIntervalInput,
  SetPropertyIntervalInput,
  StorePageRankRunInput,
  TombstoneEntityInput,
  TraverseAtTimeInput,
  TraverseEdgeItem,
  TypeDefinition,
  TypeFieldDefinition,
  ValidTime,
  Value,
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
  getProjectionEdges: 'mneme_get_projection_edges',
  getGraphDegreeStats: 'mneme_get_graph_degree_stats',
  getGraphEdgeTypeCounts: 'mneme_get_graph_edge_type_counts',
  storePageRankRun: 'mneme_store_pagerank_scores',
  getPageRankScores: 'mneme_get_pagerank_scores',
} as const;

/**
 * Coerce command payloads into the invoke arguments shape.
 * @param value - Payload to pass to the host.
 */
function toInvokeArguments(value: object): Record<string, unknown> {
  return value as Record<string, unknown>;
}

export interface UpsertMetamodelBatchInput {
  partitionId: string;
  actorId: string;
  assertedAt: AssertedTime;
  batch: MetamodelBatch;
  scenarioId?: string;
}

export interface CompileEffectiveSchemaInput {
  partitionId: string;
  actorId: string;
  assertedAt: AssertedTime;
  typeId: string;
  scenarioId?: string;
}

export interface MnemeOpResult {
  opId: string;
}

export interface EffectiveSchema {
  typeId: string;
  appliesTo: 'Node' | 'Edge';
  fields: {
    fieldId: string;
    valueType: 'str' | 'i64' | 'f64' | 'bool' | 'time' | 'ref' | 'blob' | 'json';
    cardinality: 'single' | 'multi';
    mergePolicy: 'LWW' | 'MV' | 'OR_SET' | 'COUNTER' | 'TEXT';
    required: boolean;
    defaultValue?: unknown;
    indexed: boolean;
    disallowOverlap?: boolean;
  }[];
}

/**
 * Upsert metamodel changes into the host.
 * @param input - Batch payload.
 */
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

/**
 * Compile the effective schema for a type.
 * @param input - Compile request payload.
 */
export async function compileEffectiveSchema(
  input: CompileEffectiveSchemaInput,
): Promise<SchemaCompileResult> {
  if (!isTauri()) {
    return { schemaVersionHash: 'mock-schema-hash' };
  }
  try {
    return await invoke<SchemaCompileResult>(
      COMMANDS.compileEffectiveSchema,
      toInvokeArguments(input),
    );
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.compileEffectiveSchema}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

/**
 * Create a node entity.
 * @param input - Create request payload.
 */
export async function createNode(input: CreateNodeInput): Promise<MnemeOpResult> {
  if (!isTauri()) {
    return { opId: 'mock-op' };
  }
  try {
    return await invoke<MnemeOpResult>(COMMANDS.createNode, toInvokeArguments(input));
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.createNode}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

/**
 * Create an edge entity.
 * @param input - Create request payload.
 */
export async function createEdge(input: CreateEdgeInput): Promise<MnemeOpResult> {
  if (!isTauri()) {
    return { opId: 'mock-op' };
  }
  try {
    return await invoke<MnemeOpResult>(COMMANDS.createEdge, toInvokeArguments(input));
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.createEdge}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

/**
 * Update the existence interval for an edge.
 * @param input - Update payload.
 */
export async function setEdgeExistenceInterval(
  input: SetEdgeExistenceIntervalInput,
): Promise<MnemeOpResult> {
  if (!isTauri()) {
    return { opId: 'mock-op' };
  }
  try {
    return await invoke<MnemeOpResult>(
      COMMANDS.setEdgeExistenceInterval,
      toInvokeArguments(input),
    );
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.setEdgeExistenceInterval}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

/**
 * Tombstone an entity.
 * @param input - Tombstone payload.
 */
export async function tombstoneEntity(input: TombstoneEntityInput): Promise<MnemeOpResult> {
  if (!isTauri()) {
    return { opId: 'mock-op' };
  }
  try {
    return await invoke<MnemeOpResult>(COMMANDS.tombstoneEntity, toInvokeArguments(input));
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.tombstoneEntity}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

/**
 * Set a property interval on an entity.
 * @param input - Update payload.
 */
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

/**
 * Clear a property interval on an entity.
 * @param input - Clear payload.
 */
export async function clearPropertyInterval(
  input: ClearPropertyIntervalInput,
): Promise<MnemeOpResult> {
  if (!isTauri()) {
    return { opId: 'mock-op' };
  }
  try {
    return await invoke<MnemeOpResult>(
      COMMANDS.clearPropertyInterval,
      toInvokeArguments(input),
    );
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.clearPropertyInterval}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

/**
 * Update an OR-Set value.
 * @param input - Update payload.
 */
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

/**
 * Apply a counter update.
 * @param input - Update payload.
 */
export async function counterUpdate(input: CounterUpdateInput): Promise<MnemeOpResult> {
  if (!isTauri()) {
    return { opId: 'mock-op' };
  }
  try {
    return await invoke<MnemeOpResult>(COMMANDS.counterUpdate, toInvokeArguments(input));
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.counterUpdate}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

/**
 * Read an entity state at a given time.
 * @param input - Read payload.
 */
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

/**
 * Traverse edges at a given time.
 * @param input - Traverse payload.
 */
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

/**
 * List entities at a given time.
 * @param input - List payload.
 */
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

/**
 * Fetch projection edges for analytics.
 */
export async function getProjectionEdges(
  input: GetProjectionEdgesInput,
): Promise<ProjectionEdge[]> {
  if (!isTauri()) {
    return [];
  }
  try {
    const raw = await invoke<RustProjectionEdge[]>(COMMANDS.getProjectionEdges, input);
    return raw.map(fromRustProjectionEdge);
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.getProjectionEdges}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

/**
 * Fetch graph degree stats.
 */
export async function getGraphDegreeStats(
  input: GetGraphDegreeStatsInput,
): Promise<GraphDegreeStat[]> {
  if (!isTauri()) {
    return [];
  }
  try {
    const raw = await invoke<RustGraphDegreeStat[]>(COMMANDS.getGraphDegreeStats, input);
    return raw.map(fromRustGraphDegreeStat);
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.getGraphDegreeStats}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

/**
 * Fetch edge type counts.
 */
export async function getGraphEdgeTypeCounts(
  input: GetGraphEdgeTypeCountsInput,
): Promise<GraphEdgeTypeCount[]> {
  if (!isTauri()) {
    return [];
  }
  try {
    const raw = await invoke<RustGraphEdgeTypeCount[]>(COMMANDS.getGraphEdgeTypeCounts, input);
    return raw.map(fromRustGraphEdgeTypeCount);
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.getGraphEdgeTypeCounts}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

/**
 * Store PageRank scores in Mneme.
 */
export async function storePageRankRun(
  input: StorePageRankRunInput,
): Promise<PageRankRunResult> {
  if (!isTauri()) {
    return { runId: 'mock-run' };
  }
  try {
    return await invoke<PageRankRunResult>(COMMANDS.storePageRankRun, {
      ...input,
      params: toRustPageRankParams(input.params),
    });
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.storePageRankRun}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

/**
 * Fetch PageRank scores.
 */
export async function getPageRankScores(
  input: GetPageRankScoresInput,
): Promise<PageRankScore[]> {
  if (!isTauri()) {
    return [];
  }
  try {
    return await invoke<PageRankScore[]>(COMMANDS.getPageRankScores, input);
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.getPageRankScores}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

/**
 * Fetch the effective schema for a type.
 * @param partitionId - Target partition id.
 * @param typeId - Type identifier to resolve.
 */
export async function getEffectiveSchema(
  partitionId: string,
  typeId: string,
): Promise<EffectiveSchema | undefined> {
  if (!isTauri()) {
    return undefined;
  }
  try {
    const raw = await invoke<RustEffectiveSchema | null>(COMMANDS.getEffectiveSchema, {
      partitionId,
      typeId,
    });
    return raw ? fromRustEffectiveSchema(raw) : undefined;
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.getEffectiveSchema}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

/**
 * List edge type rules for the partition.
 * @param partitionId - Target partition id.
 * @param edgeTypeId - Optional edge type filter.
 */
export async function listEdgeTypeRules(
  partitionId: string,
  edgeTypeId?: string,
): Promise<EdgeTypeRuleDefinition[]> {
  if (!isTauri()) {
    return [];
  }
  try {
    const raw = await invoke<RustEdgeTypeRule[]>(COMMANDS.listEdgeTypeRules, {
      partitionId,
      edgeTypeId,
    });
    return raw.map((rule) => fromRustEdgeTypeRule(rule));
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.listEdgeTypeRules}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

type RustValueType = 'Str' | 'I64' | 'F64' | 'Bool' | 'Time' | 'Ref' | 'Blob' | 'Json';
type RustMergePolicy = 'Lww' | 'Mv' | 'OrSet' | 'Counter' | 'Text';

interface RustTypeDefinition {
  type_id: string;
  applies_to: 'Node' | 'Edge';
  label: string;
  is_abstract: boolean;
  parent_type_id?: string;
}

interface RustFieldDefinition {
  field_id: string;
  label: string;
  value_type: RustValueType;
  cardinality_multi: boolean;
  merge_policy: RustMergePolicy;
  is_indexed: boolean;
}

interface RustTypeFieldDefinition {
  type_id: string;
  field_id: string;
  is_required: boolean;
  default_value?: unknown;
  override_default?: boolean;
  tighten_required?: boolean;
}

interface RustEdgeTypeRule {
  edge_type_id: string;
  allowed_src_type_ids: string[];
  allowed_dst_type_ids: string[];
  semantic_direction?: string;
}

interface RustMetamodelBatch {
  types: RustTypeDefinition[];
  fields: RustFieldDefinition[];
  type_fields: RustTypeFieldDefinition[];
  edge_type_rules: RustEdgeTypeRule[];
  metamodel_version?: string;
  metamodel_source?: string;
}

interface RustEffectiveSchemaField {
  field_id: string;
  value_type: RustValueType;
  cardinality_multi: boolean;
  merge_policy: RustMergePolicy;
  is_required: boolean;
  default_value?: unknown;
  is_indexed: boolean;
  disallow_overlap: boolean;
}

interface RustEffectiveSchema {
  type_id: string;
  applies_to: 'Node' | 'Edge';
  fields: RustEffectiveSchemaField[];
}

interface RustReadEntityAtTimeResult {
  entity_id: string;
  kind: 'Node' | 'Edge';
  type_id?: string;
  is_deleted: boolean;
  properties: Record<string, RustReadValue>;
}

interface RustTraverseEdgeItem {
  edge_id: string;
  src_id: string;
  dst_id: string;
  type_id?: string;
}

interface RustListEntitiesResultItem {
  entity_id: string;
  kind: 'Node' | 'Edge';
  type_id?: string;
}

interface RustProjectionEdge {
  edge_id: string;
  src_id: string;
  dst_id: string;
  edge_type_id?: string;
  weight: number;
}

interface RustGraphDegreeStat {
  entity_id: string;
  out_degree: number;
  in_degree: number;
  as_of_valid_time?: number | null;
  computed_asserted_at: number;
}

interface RustGraphEdgeTypeCount {
  edge_type_id?: string | null;
  count: number;
  computed_asserted_at: number;
}

interface RustPageRankSeed {
  id: string;
  weight: number;
}

interface RustPageRankParams {
  damping: number;
  maxIters: number;
  tol: number;
  personalisedSeed?: RustPageRankSeed[];
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

/**
 * Normalize value types to the Rust enum representation.
 * @param valueType - Value type in either Rust or API form.
 */
function toRustValueType(valueType: RustValueType | EffectiveSchema['fields'][number]['valueType']) {
  if (valueType in VALUE_TYPE_MAP) {
    return VALUE_TYPE_MAP[valueType as EffectiveSchema['fields'][number]['valueType']];
  }
  return valueType;
}

/**
 * Normalize merge policies to the Rust enum representation.
 * @param mergePolicy - Merge policy in either Rust or API form.
 */
function toRustMergePolicy(
  mergePolicy: RustMergePolicy | EffectiveSchema['fields'][number]['mergePolicy'],
) {
  if (mergePolicy in MERGE_POLICY_MAP) {
    return MERGE_POLICY_MAP[mergePolicy as EffectiveSchema['fields'][number]['mergePolicy']];
  }
  return mergePolicy;
}

/**
 * Convert a TypeDefinition into the Rust representation.
 * @param definition - Type definition to convert.
 */
function toRustType(definition: TypeDefinition): RustTypeDefinition {
  return {
    type_id: definition.typeId,
    applies_to: definition.appliesTo,
    label: definition.label,
    is_abstract: definition.isAbstract,
    parent_type_id: definition.parentTypeId,
  };
}

/**
 * Convert a FieldDefinition into the Rust representation.
 * @param definition - Field definition to convert.
 */
function toRustField(definition: FieldDefinition): RustFieldDefinition {
  return {
    field_id: definition.fieldId,
    label: definition.label,
    value_type: toRustValueType(definition.valueType),
    cardinality_multi: definition.cardinality === 'multi',
    merge_policy: toRustMergePolicy(definition.mergePolicy),
    is_indexed: definition.indexed,
  };
}

/**
 * Convert a TypeFieldDefinition into the Rust representation.
 * @param definition - Type field definition to convert.
 */
function toRustTypeField(definition: TypeFieldDefinition): RustTypeFieldDefinition {
  return {
    type_id: definition.typeId,
    field_id: definition.fieldId,
    is_required: definition.required,
    default_value: definition.defaultValue,
    override_default: definition.overrideDefault,
    tighten_required: definition.tightenRequired,
  };
}

/**
 * Convert an EdgeTypeRuleDefinition into the Rust representation.
 * @param definition - Edge type rule definition to convert.
 */
function toRustEdgeTypeRule(definition: EdgeTypeRuleDefinition): RustEdgeTypeRule {
  return {
    edge_type_id: definition.edgeTypeId,
    semantic_direction: definition.semanticDirection,
    allowed_src_type_ids: definition.allowedSrcTypeIds ?? [],
    allowed_dst_type_ids: definition.allowedDstTypeIds ?? [],
  };
}

/**
 * Convert the metamodel batch into Rust-friendly payloads.
 * @param batch - Batch to convert.
 */
function toRustMetamodelBatch(batch: MetamodelBatch): RustMetamodelBatch {
  return {
    types: batch.types.map((typeDef) => toRustType(typeDef)),
    fields: batch.fields.map((fieldDef) => toRustField(fieldDef)),
    type_fields: batch.typeFields.map((typeField) => toRustTypeField(typeField)),
    edge_type_rules: (batch.edgeTypeRules ?? []).map((rule) => toRustEdgeTypeRule(rule)),
    metamodel_version: undefined,
    metamodel_source: undefined,
  };
}

/**
 * Convert a field filter into the Rust wire format.
 * @param filter - Filter input to convert.
 */
function toRustFieldFilter(filter: FieldFilter) {
  return {
    fieldId: filter.fieldId,
    op: filter.op,
    value: toRustValue(filter.value),
  };
}

/**
 * Convert a value payload into the Rust wire format.
 * @param value - Value to convert.
 */
function toRustValue(value: Value): RustValue {
  switch (value.t) {
    case 'str': {
      return { Str: value.v };
    }
    case 'i64': {
      const numberValue = Number(value.v);
      if (!Number.isSafeInteger(numberValue)) {
        throw new TypeError('Value.i64 exceeds safe integer range for IPC transport.');
      }
      return { I64: numberValue };
    }
    case 'f64': {
      return { F64: value.v };
    }
    case 'bool': {
      return { Bool: value.v };
    }
    case 'time': {
      return { Time: toValidTimeMicros(value.v) };
    }
    case 'ref': {
      return { Ref: value.v };
    }
    case 'blob': {
      return { Blob: value.v };
    }
    case 'json': {
      return { Json: value.v };
    }
    default: {
      const _exhaustive: never = value;
      throw new TypeError(`Unsupported value type: ${String(_exhaustive)}`);
    }
  }
}

/**
 * Convert an ISO timestamp to microseconds.
 * @param value - ISO-8601 timestamp.
 */
function toValidTimeMicros(value: ValidTime): number {
  const parsed = Date.parse(value);
  if (Number.isNaN(parsed)) {
    throw new TypeError(`Invalid ISO-8601 timestamp: ${value}`);
  }
  return parsed * 1000;
}

function fromValidTimeMicros(value?: number | null): ValidTime | undefined {
  if (value === undefined || value === null) {
    return undefined;
  }
  return new Date(value / 1000).toISOString() as ValidTime;
}

function hlcToString(value: number): AssertedTime {
  return `${value}` as AssertedTime;
}

function toRustPageRankParams(params: PageRankRunParams): RustPageRankParams {
  return {
    ...params,
    personalisedSeed: params.personalisedSeed?.map((seed) => ({
      id: seed.id,
      weight: seed.w,
    })),
  };
}

/**
 * Convert a Rust value into the API representation.
 * @param value - Rust value to convert.
 */
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
  throw new TypeError('Unsupported Rust value variant');
}

/**
 * Convert a Rust read result into the API representation.
 * @param raw - Rust payload.
 */
function fromRustReadEntityAtTime(raw: RustReadEntityAtTimeResult): ReadEntityAtTimeResult {
  const properties: Record<string, ReadValue> = {};
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

/**
 * Convert a Rust read value into the API representation.
 * @param value - Rust read value to convert.
 */
function toReadValue(value: RustReadValue): ReadValue {
  if ('Single' in value) {
    return { k: 'single', v: fromRustValue(value.Single) };
  }
  if ('Multi' in value) {
    return { k: 'multi', v: value.Multi.map((item) => fromRustValue(item)) };
  }
  return {
    k: 'multi_limited',
    v: {
      values: value.MultiLimited.values.map((item) => fromRustValue(item)),
      moreAvailable: value.MultiLimited.more_available,
    },
  };
}

function fromRustProjectionEdge(edge: RustProjectionEdge): ProjectionEdge {
  return {
    edgeId: edge.edge_id,
    srcId: edge.src_id,
    dstId: edge.dst_id,
    edgeTypeId: edge.edge_type_id ?? undefined,
    weight: edge.weight,
  };
}

function fromRustGraphDegreeStat(stat: RustGraphDegreeStat): GraphDegreeStat {
  return {
    entityId: stat.entity_id,
    outDegree: stat.out_degree,
    inDegree: stat.in_degree,
    asOfValidTime: fromValidTimeMicros(stat.as_of_valid_time),
    computedAssertedAt: hlcToString(stat.computed_asserted_at),
  };
}

function fromRustGraphEdgeTypeCount(count: RustGraphEdgeTypeCount): GraphEdgeTypeCount {
  return {
    edgeTypeId: count.edge_type_id ?? undefined,
    count: count.count,
    computedAssertedAt: hlcToString(count.computed_asserted_at),
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

/**
 * Convert a Rust effective schema into the API representation.
 * @param schema - Rust schema payload.
 */
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

/**
 * Convert a Rust edge type rule into the API representation.
 * @param rule - Rust edge type rule.
 */
function fromRustEdgeTypeRule(rule: RustEdgeTypeRule): EdgeTypeRuleDefinition {
  return {
    edgeTypeId: rule.edge_type_id,
    semanticDirection: rule.semantic_direction ?? '',
    allowedSrcTypeIds: rule.allowed_src_type_ids,
    allowedDstTypeIds: rule.allowed_dst_type_ids,
  };
}
