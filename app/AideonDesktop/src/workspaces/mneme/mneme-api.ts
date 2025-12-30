import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type {
  AssertedTime,
  ChangeEvent,
  ClearPropertyIntervalInput,
  ComputedCacheEntry,
  ComputedRule,
  CounterUpdateInput,
  CreateEdgeInput,
  CreateNodeInput,
  CreateScenarioInput,
  DeleteScenarioInput,
  EdgeTypeRuleDefinition,
  ExplainResolutionInput,
  ExplainResolutionResult,
  ExplainTraversalInput,
  ExplainTraversalResult,
  ExportOpsInput,
  ExportOpsResult,
  ExportOptions,
  ExportRecord,
  FieldDefinition,
  FieldFilter,
  GetChangesSinceInput,
  GetGraphDegreeStatsInput,
  GetGraphEdgeTypeCountsInput,
  GetPageRankScoresInput,
  GetProjectionEdgesInput,
  GraphDegreeStat,
  GraphEdgeTypeCount,
  ImportOptions,
  ImportReport,
  IngestOpsInput,
  IntegrityHead,
  JobSummary,
  ListComputedCacheInput,
  ListEntitiesInput,
  ListEntitiesResultItem,
  MetamodelBatch,
  OpEnvelope,
  OrSetUpdateInput,
  PageRankRunParams,
  PageRankRunResult,
  PageRankScore,
  GetPartitionHeadInput,
  PartitionHeadResult,
  ProjectionEdge,
  ReadEntityAtTimeInput,
  ReadEntityAtTimeResult,
  ReadValue,
  RunProcessingWorkerInput,
  RunProcessingWorkerResult,
  SchemaCompileResult,
  SchemaHead,
  SchemaManifest,
  SetEdgeExistenceIntervalInput,
  SetPropertyIntervalInput,
  SnapshotOptions,
  StorePageRankRunInput,
  SubscribePartitionInput,
  SubscriptionResult,
  TombstoneEntityInput,
  TraverseAtTimeInput,
  TraverseEdgeItem,
  TriggerCompactionInput,
  TriggerProcessingInput,
  TriggerRetentionInput,
  TypeDefinition,
  TypeFieldDefinition,
  UnsubscribePartitionInput,
  UpsertComputedCacheInput,
  UpsertComputedRulesInput,
  UpsertValidationRulesInput,
  ValidationRule,
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
  exportOps: 'mneme_export_ops',
  ingestOps: 'mneme_ingest_ops',
  getPartitionHead: 'mneme_get_partition_head',
  createScenario: 'mneme_create_scenario',
  deleteScenario: 'mneme_delete_scenario',
  exportOpsStream: 'mneme_export_ops_stream',
  importOpsStream: 'mneme_import_ops_stream',
  exportSnapshotStream: 'mneme_export_snapshot_stream',
  importSnapshotStream: 'mneme_import_snapshot_stream',
  upsertValidationRules: 'mneme_upsert_validation_rules',
  listValidationRules: 'mneme_list_validation_rules',
  upsertComputedRules: 'mneme_upsert_computed_rules',
  listComputedRules: 'mneme_list_computed_rules',
  upsertComputedCache: 'mneme_upsert_computed_cache',
  listComputedCache: 'mneme_list_computed_cache',
  triggerRebuildEffectiveSchema: 'mneme_trigger_rebuild_effective_schema',
  triggerRefreshIntegrity: 'mneme_trigger_refresh_integrity',
  triggerRefreshAnalyticsProjections: 'mneme_trigger_refresh_analytics_projections',
  triggerRetention: 'mneme_trigger_retention',
  triggerCompaction: 'mneme_trigger_compaction',
  runProcessingWorker: 'mneme_run_processing_worker',
  listJobs: 'mneme_list_jobs',
  getChangesSince: 'mneme_get_changes_since',
  subscribePartition: 'mneme_subscribe_partition',
  unsubscribePartition: 'mneme_unsubscribe_partition',
  getIntegrityHead: 'mneme_get_integrity_head',
  getLastSchemaCompile: 'mneme_get_last_schema_compile',
  listFailedJobs: 'mneme_list_failed_jobs',
  getSchemaManifest: 'mneme_get_schema_manifest',
  explainResolution: 'mneme_explain_resolution',
  explainTraversal: 'mneme_explain_traversal',
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
export async function upsertMetamodelBatch(
  input: UpsertMetamodelBatchInput,
): Promise<MnemeOpResult> {
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
    return await invoke<MnemeOpResult>(COMMANDS.setEdgeExistenceInterval, toInvokeArguments(input));
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.setEdgeExistenceInterval}' failed: ${String(error)}`,
      {
        cause: error,
      },
    );
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
    return await invoke<MnemeOpResult>(COMMANDS.clearPropertyInterval, toInvokeArguments(input));
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
export async function traverseAtTime(input: TraverseAtTimeInput): Promise<TraverseEdgeItem[]> {
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
export async function listEntities(input: ListEntitiesInput): Promise<ListEntitiesResultItem[]> {
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
export async function storePageRankRun(input: StorePageRankRunInput): Promise<PageRankRunResult> {
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
export async function getPageRankScores(input: GetPageRankScoresInput): Promise<PageRankScore[]> {
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
 * Export op log entries.
 */
export async function exportOps(input: ExportOpsInput): Promise<ExportOpsResult> {
  if (!isTauri()) {
    return { ops: [] };
  }
  try {
    const raw = await invoke<RustOpEnvelope[]>(COMMANDS.exportOps, input);
    return { ops: raw.map(fromRustOpEnvelope) };
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.exportOps}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

/**
 * Ingest op log entries.
 */
export async function ingestOps(input: IngestOpsInput): Promise<void> {
  if (!isTauri()) {
    return;
  }
  try {
    await invoke<void>(COMMANDS.ingestOps, {
      ...input,
      ops: input.ops.map(toRustOpEnvelope),
    });
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.ingestOps}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

/**
 * Fetch the partition head.
 */
export async function getPartitionHead(
  input: GetPartitionHeadInput,
): Promise<PartitionHeadResult> {
  if (!isTauri()) {
    return { head: '0' };
  }
  try {
    return await invoke<PartitionHeadResult>(COMMANDS.getPartitionHead, input);
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.getPartitionHead}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

/**
 * Create a scenario overlay.
 */
export async function createScenario(input: CreateScenarioInput): Promise<string> {
  if (!isTauri()) {
    return 'mock-scenario';
  }
  try {
    return await invoke<string>(COMMANDS.createScenario, toInvokeArguments(input));
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.createScenario}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

/**
 * Delete a scenario overlay.
 */
export async function deleteScenario(input: DeleteScenarioInput): Promise<void> {
  if (!isTauri()) {
    return;
  }
  try {
    await invoke<void>(COMMANDS.deleteScenario, toInvokeArguments(input));
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.deleteScenario}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

/**
 * Export a streaming op log payload.
 */
export async function exportOpsStream(
  options: ExportOptions,
): Promise<AsyncIterable<ExportRecord>> {
  if (!isTauri()) {
    return toAsyncIterable([]);
  }
  try {
    const raw = await invoke<RustExportRecord[]>(COMMANDS.exportOpsStream, options);
    return toAsyncIterable(raw.map(fromRustExportRecord));
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.exportOpsStream}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

/**
 * Import a streaming op log payload.
 */
export async function importOpsStream(
  options: ImportOptions,
  records: AsyncIterable<ExportRecord>,
): Promise<ImportReport> {
  if (!isTauri()) {
    return { opsImported: 0, opsSkipped: 0, errors: 0 };
  }
  try {
    const collected = await collectAsyncIterable(records);
    const raw = await invoke<RustImportReport>(COMMANDS.importOpsStream, {
      ...options,
      records: collected.map(toRustExportRecord),
    });
    return fromRustImportReport(raw);
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.importOpsStream}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

/**
 * Export a snapshot stream.
 */
export async function exportSnapshotStream(
  options: SnapshotOptions,
): Promise<AsyncIterable<ExportRecord>> {
  if (!isTauri()) {
    return toAsyncIterable([]);
  }
  try {
    const raw = await invoke<RustExportRecord[]>(COMMANDS.exportSnapshotStream, options);
    return toAsyncIterable(raw.map(fromRustExportRecord));
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.exportSnapshotStream}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

/**
 * Import a snapshot stream.
 */
export async function importSnapshotStream(
  options: ImportOptions,
  records: AsyncIterable<ExportRecord>,
): Promise<void> {
  if (!isTauri()) {
    return;
  }
  try {
    const collected = await collectAsyncIterable(records);
    await invoke<void>(COMMANDS.importSnapshotStream, {
      ...options,
      records: collected.map(toRustExportRecord),
    });
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.importSnapshotStream}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export const mnemeExportApi = {
  exportOps: exportOpsStream,
};

export const mnemeImportApi = {
  importOps: importOpsStream,
};

export const mnemeSnapshotApi = {
  exportSnapshotStream,
  importSnapshotStream,
};

export const mnemeMetamodelApi = {
  upsertMetamodelBatch,
  compileEffectiveSchema,
};

export const mnemeWriteApi = {
  createNode,
  createEdge,
  setEdgeExistenceInterval,
  tombstoneEntity,
  setPropertyInterval,
  clearPropertyInterval,
  orSetUpdate,
  counterUpdate,
};

export const mnemeReadApi = {
  readEntityAtTime,
  traverseAtTime,
  listEntities,
};

export const mnemeAnalyticsApi = {
  getProjectionEdges,
  getGraphDegreeStats,
  getGraphEdgeTypeCounts,
  storePageRankRun,
  getPageRankScores,
};

export const mnemeSyncApi = {
  exportOps,
  ingestOps,
  getPartitionHead,
};

export const mnemeProcessingApi = {
  triggerRebuildEffectiveSchema,
  triggerRefreshIntegrity,
  triggerRefreshAnalyticsProjections,
  triggerRetention,
  triggerCompaction,
  runProcessingWorker,
  listJobs,
};

/**
 * Upsert validation rules for a partition.
 */
export async function upsertValidationRules(input: UpsertValidationRulesInput): Promise<void> {
  if (!isTauri()) {
    return;
  }
  try {
    await invoke<void>(COMMANDS.upsertValidationRules, {
      ...input,
      rules: input.rules.map(toRustValidationRule),
    });
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.upsertValidationRules}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

/**
 * List validation rules for a partition.
 */
export async function listValidationRules(partitionId: string): Promise<ValidationRule[]> {
  if (!isTauri()) {
    return [];
  }
  try {
    const raw = await invoke<RustValidationRule[]>(COMMANDS.listValidationRules, { partitionId });
    return raw.map(fromRustValidationRule);
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.listValidationRules}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

/**
 * Upsert computed rules for a partition.
 */
export async function upsertComputedRules(input: UpsertComputedRulesInput): Promise<void> {
  if (!isTauri()) {
    return;
  }
  try {
    await invoke<void>(COMMANDS.upsertComputedRules, {
      ...input,
      rules: input.rules.map(toRustComputedRule),
    });
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.upsertComputedRules}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

/**
 * List computed rules for a partition.
 */
export async function listComputedRules(partitionId: string): Promise<ComputedRule[]> {
  if (!isTauri()) {
    return [];
  }
  try {
    const raw = await invoke<RustComputedRule[]>(COMMANDS.listComputedRules, { partitionId });
    return raw.map(fromRustComputedRule);
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.listComputedRules}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

/**
 * Upsert computed cache entries for a partition.
 */
export async function upsertComputedCache(input: UpsertComputedCacheInput): Promise<void> {
  if (!isTauri()) {
    return;
  }
  try {
    await invoke<void>(COMMANDS.upsertComputedCache, {
      partitionId: input.partitionId,
      entries: input.entries.map(toRustComputedCacheEntry),
    });
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.upsertComputedCache}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

/**
 * List computed cache entries.
 */
export async function listComputedCache(
  input: ListComputedCacheInput,
): Promise<ComputedCacheEntry[]> {
  if (!isTauri()) {
    return [];
  }
  try {
    const raw = await invoke<RustComputedCacheEntry[]>(COMMANDS.listComputedCache, input);
    return raw.map(fromRustComputedCacheEntry);
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.listComputedCache}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function triggerRebuildEffectiveSchema(input: TriggerProcessingInput): Promise<void> {
  if (!isTauri()) {
    return;
  }
  try {
    await invoke<void>(COMMANDS.triggerRebuildEffectiveSchema, input);
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.triggerRebuildEffectiveSchema}' failed: ${String(error)}`,
      { cause: error },
    );
  }
}

export async function triggerRefreshIntegrity(input: TriggerProcessingInput): Promise<void> {
  if (!isTauri()) {
    return;
  }
  try {
    await invoke<void>(COMMANDS.triggerRefreshIntegrity, input);
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.triggerRefreshIntegrity}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function triggerRefreshAnalyticsProjections(
  input: TriggerProcessingInput,
): Promise<void> {
  if (!isTauri()) {
    return;
  }
  try {
    await invoke<void>(COMMANDS.triggerRefreshAnalyticsProjections, input);
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.triggerRefreshAnalyticsProjections}' failed: ${String(error)}`,
      { cause: error },
    );
  }
}

export async function triggerRetention(input: TriggerRetentionInput): Promise<void> {
  if (!isTauri()) {
    return;
  }
  try {
    await invoke<void>(COMMANDS.triggerRetention, input);
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.triggerRetention}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function triggerCompaction(input: TriggerCompactionInput): Promise<void> {
  if (!isTauri()) {
    return;
  }
  try {
    await invoke<void>(COMMANDS.triggerCompaction, input);
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.triggerCompaction}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function runProcessingWorker(
  input: RunProcessingWorkerInput,
): Promise<RunProcessingWorkerResult> {
  if (!isTauri()) {
    return { jobsProcessed: 0 };
  }
  try {
    return await invoke<RunProcessingWorkerResult>(COMMANDS.runProcessingWorker, input);
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.runProcessingWorker}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function listJobs(input: ListJobsInput): Promise<JobSummary[]> {
  if (!isTauri()) {
    return [];
  }
  try {
    const raw = await invoke<RustJobSummary[]>(COMMANDS.listJobs, input);
    return raw.map(fromRustJobSummary);
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.listJobs}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function getChangesSince(input: GetChangesSinceInput): Promise<ChangeEvent[]> {
  if (!isTauri()) {
    return [];
  }
  try {
    const raw = await invoke<RustChangeEvent[]>(COMMANDS.getChangesSince, input);
    return raw.map(fromRustChangeEvent);
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.getChangesSince}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function subscribePartition(
  input: SubscribePartitionInput,
): Promise<SubscriptionResult> {
  if (!isTauri()) {
    return { subscriptionId: 'mock-sub' };
  }
  try {
    return await invoke<SubscriptionResult>(COMMANDS.subscribePartition, input);
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.subscribePartition}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function unsubscribePartition(input: UnsubscribePartitionInput): Promise<boolean> {
  if (!isTauri()) {
    return true;
  }
  try {
    return await invoke<boolean>(COMMANDS.unsubscribePartition, input);
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.unsubscribePartition}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function onChangeEvents(handler: (event: ChangeEvent) => void): Promise<() => void> {
  if (!isTauri()) {
    return async () => {};
  }
  const unlisten = await listen<RustChangeEvent>('mneme_change_event', (event) => {
    handler(fromRustChangeEvent(event.payload));
  });
  return unlisten;
}

export async function getIntegrityHead(
  partitionId: string,
  scenarioId?: string,
): Promise<IntegrityHead | undefined> {
  if (!isTauri()) {
    return undefined;
  }
  try {
    const raw = await invoke<RustIntegrityHead | null>(COMMANDS.getIntegrityHead, {
      partitionId,
      scenarioId,
    });
    return raw ? fromRustIntegrityHead(raw) : undefined;
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.getIntegrityHead}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function getLastSchemaCompile(
  partitionId: string,
  typeId: string,
): Promise<SchemaHead | undefined> {
  if (!isTauri()) {
    return undefined;
  }
  try {
    const raw = await invoke<RustSchemaHead | null>(COMMANDS.getLastSchemaCompile, {
      partitionId,
      typeId,
    });
    return raw ? fromRustSchemaHead(raw) : undefined;
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.getLastSchemaCompile}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function listFailedJobs(partitionId: string, limit: number): Promise<JobSummary[]> {
  if (!isTauri()) {
    return [];
  }
  try {
    const raw = await invoke<RustJobSummary[]>(COMMANDS.listFailedJobs, { partitionId, limit });
    return raw.map(fromRustJobSummary);
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.listFailedJobs}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function getSchemaManifest(): Promise<SchemaManifest> {
  if (!isTauri()) {
    return { manifestVersion: '0', migrations: [], tables: [] };
  }
  try {
    const raw = await invoke<RustSchemaManifest>(COMMANDS.getSchemaManifest);
    return fromRustSchemaManifest(raw);
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.getSchemaManifest}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function explainResolution(
  input: ExplainResolutionInput,
): Promise<ExplainResolutionResult> {
  if (!isTauri()) {
    return {
      entityId: input.entityId,
      fieldId: input.fieldId,
      candidates: [],
    };
  }
  try {
    const raw = await invoke<RustExplainResolutionResult>(COMMANDS.explainResolution, input);
    return fromRustExplainResolution(raw);
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.explainResolution}' failed: ${String(error)}`, {
      cause: error,
    });
  }
}

export async function explainTraversal(
  input: ExplainTraversalInput,
): Promise<ExplainTraversalResult> {
  if (!isTauri()) {
    return {
      edgeId: input.edgeId,
      active: false,
      candidates: [],
    };
  }
  try {
    const raw = await invoke<RustExplainTraversalResult>(COMMANDS.explainTraversal, input);
    return fromRustExplainTraversal(raw);
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.explainTraversal}' failed: ${String(error)}`, {
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

interface RustOpEnvelope {
  op_id: string;
  actor_id: string;
  asserted_at: number;
  op_type: number;
  payload: number[];
  deps: string[];
}

interface RustExportRecord {
  record_type: string;
  data: unknown;
}

interface RustImportReport {
  ops_imported: number;
  ops_skipped: number;
  errors: number;
}

interface RustValidationRule {
  rule_id: string;
  scope_kind: number;
  scope_id?: string | null;
  severity: number;
  template_kind: string;
  params: unknown;
}

interface RustComputedRule {
  rule_id: string;
  target_type_id?: string | null;
  output_field_id?: string | null;
  template_kind: string;
  params: unknown;
}

interface RustComputedCacheEntry {
  entity_id: string;
  field_id: string;
  valid_from: number;
  valid_to?: number | null;
  value: RustValue;
  rule_version_hash: string;
  computed_asserted_at: number;
}

interface RustComputedCacheEntryPayload {
  entity_id: string;
  field_id: string;
  valid_from: string;
  valid_to?: string | null;
  value: RustValue;
  rule_version_hash: string;
  computed_asserted_at: string;
}

interface RustJobSummary {
  partition: string;
  job_id: string;
  job_type: string;
  status: number;
  priority: number;
  attempts: number;
  max_attempts: number;
  lease_expires_at?: number | null;
  next_run_after?: number | null;
  created_asserted_at: number;
  updated_asserted_at: number;
  dedupe_key?: string | null;
  last_error?: string | null;
}

interface RustChangeEvent {
  partition: string;
  sequence: number;
  op_id: string;
  asserted_at: number;
  entity_id?: string | null;
  change_kind: number;
  payload?: unknown;
}

interface RustIntegrityHead {
  partition: string;
  scenario_id?: string | null;
  run_id: string;
  updated_asserted_at: number;
}

interface RustSchemaHead {
  partition: string;
  type_id: string;
  schema_version_hash: string;
  updated_asserted_at: number;
}

interface RustSchemaManifest {
  manifest_version: string;
  migrations: string[];
  tables: RustTableManifest[];
}

interface RustTableManifest {
  name: string;
  columns: RustColumnManifest[];
  indexes: RustIndexManifest[];
}

interface RustColumnManifest {
  name: string;
  logical_type: string;
  nullable: boolean;
}

interface RustIndexManifest {
  name: string;
  columns: string[];
  unique: boolean;
}

interface RustExplainPrecedence {
  layer: number;
  interval_width: number;
  asserted_at: number;
  op_id: string;
}

interface RustExplainPropertyFact {
  value: RustValue;
  valid_from: number;
  valid_to?: number | null;
  layer: number;
  asserted_at: number;
  op_id: string;
  is_tombstone: boolean;
  precedence: RustExplainPrecedence;
}

interface RustExplainEdgeFact {
  edge_id: string;
  src_id: string;
  dst_id: string;
  edge_type_id?: string | null;
  valid_from: number;
  valid_to?: number | null;
  layer: number;
  asserted_at: number;
  op_id: string;
  is_tombstone: boolean;
  precedence: RustExplainPrecedence;
}

interface RustExplainResolutionResult {
  entity_id: string;
  field_id: string;
  resolved?: RustReadValue | null;
  winner?: RustExplainPropertyFact | null;
  candidates: RustExplainPropertyFact[];
}

interface RustExplainTraversalResult {
  edge_id: string;
  active: boolean;
  winner?: RustExplainEdgeFact | null;
  candidates: RustExplainEdgeFact[];
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

const MERGE_POLICY_MAP: Record<EffectiveSchema['fields'][number]['mergePolicy'], RustMergePolicy> =
  {
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
function toRustValueType(
  valueType: RustValueType | EffectiveSchema['fields'][number]['valueType'],
) {
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

function normalizeBytes(payload: Uint8Array | number[]): Uint8Array {
  if (payload instanceof Uint8Array) {
    return payload;
  }
  return Uint8Array.from(payload);
}

function fromRustOpEnvelope(op: RustOpEnvelope): OpEnvelope {
  return {
    opId: op.op_id,
    actorId: op.actor_id,
    assertedAt: hlcToString(op.asserted_at),
    opType: op.op_type,
    payload: normalizeBytes(op.payload),
    deps: op.deps,
  };
}

function toRustOpEnvelope(op: OpEnvelope) {
  return {
    opId: op.opId,
    actorId: op.actorId,
    assertedAt: op.assertedAt,
    opType: op.opType,
    payload: Array.from(op.payload),
    deps: op.deps,
  };
}

function fromRustExportRecord(record: RustExportRecord): ExportRecord {
  return {
    recordType: record.record_type,
    data: record.data,
  };
}

function toRustExportRecord(record: ExportRecord): RustExportRecord {
  return {
    record_type: record.recordType,
    data: record.data,
  };
}

function fromRustImportReport(report: RustImportReport): ImportReport {
  return {
    opsImported: report.ops_imported,
    opsSkipped: report.ops_skipped,
    errors: report.errors,
  };
}

function fromRustValidationRule(rule: RustValidationRule): ValidationRule {
  return {
    ruleId: rule.rule_id,
    scopeKind: rule.scope_kind,
    scopeId: rule.scope_id ?? undefined,
    severity: rule.severity,
    templateKind: rule.template_kind,
    params: rule.params,
  };
}

function toRustValidationRule(rule: ValidationRule): RustValidationRule {
  return {
    rule_id: rule.ruleId,
    scope_kind: rule.scopeKind,
    scope_id: rule.scopeId ?? null,
    severity: rule.severity,
    template_kind: rule.templateKind,
    params: rule.params,
  };
}

function fromRustComputedRule(rule: RustComputedRule): ComputedRule {
  return {
    ruleId: rule.rule_id,
    targetTypeId: rule.target_type_id ?? undefined,
    outputFieldId: rule.output_field_id ?? undefined,
    templateKind: rule.template_kind,
    params: rule.params,
  };
}

function toRustComputedRule(rule: ComputedRule): RustComputedRule {
  return {
    rule_id: rule.ruleId,
    target_type_id: rule.targetTypeId ?? null,
    output_field_id: rule.outputFieldId ?? null,
    template_kind: rule.templateKind,
    params: rule.params,
  };
}

function fromRustComputedCacheEntry(entry: RustComputedCacheEntry): ComputedCacheEntry {
  return {
    entityId: entry.entity_id,
    fieldId: entry.field_id,
    validFrom: fromValidTimeMicros(entry.valid_from) ?? '',
    validTo: fromValidTimeMicros(entry.valid_to),
    value: fromRustValue(entry.value),
    ruleVersionHash: entry.rule_version_hash,
    computedAssertedAt: hlcToString(entry.computed_asserted_at),
  };
}

function toRustComputedCacheEntry(entry: ComputedCacheEntry): RustComputedCacheEntryPayload {
  return {
    entity_id: entry.entityId,
    field_id: entry.fieldId,
    valid_from: entry.validFrom,
    valid_to: entry.validTo ?? null,
    value: toRustValue(entry.value),
    rule_version_hash: entry.ruleVersionHash,
    computed_asserted_at: entry.computedAssertedAt,
  };
}

async function* toAsyncIterable<T>(items: T[]): AsyncIterable<T> {
  for (const item of items) {
    yield item;
  }
}

async function collectAsyncIterable<T>(items: AsyncIterable<T>): Promise<T[]> {
  const collected: T[] = [];
  for await (const item of items) {
    collected.push(item);
  }
  return collected;
}

function fromRustJobSummary(job: RustJobSummary): JobSummary {
  return {
    partitionId: job.partition,
    jobId: job.job_id,
    jobType: job.job_type,
    status: job.status,
    priority: job.priority,
    attempts: job.attempts,
    maxAttempts: job.max_attempts,
    leaseExpiresAt: job.lease_expires_at ?? undefined,
    nextRunAfter: job.next_run_after ?? undefined,
    createdAssertedAt: hlcToString(job.created_asserted_at),
    updatedAssertedAt: hlcToString(job.updated_asserted_at),
    dedupeKey: job.dedupe_key ?? undefined,
    lastError: job.last_error ?? undefined,
  };
}

function fromRustChangeEvent(event: RustChangeEvent): ChangeEvent {
  return {
    partitionId: event.partition,
    sequence: event.sequence,
    opId: event.op_id,
    assertedAt: hlcToString(event.asserted_at),
    entityId: event.entity_id ?? undefined,
    changeKind: event.change_kind,
    payload: event.payload,
  };
}

function fromRustIntegrityHead(head: RustIntegrityHead): IntegrityHead {
  return {
    partitionId: head.partition,
    scenarioId: head.scenario_id ?? undefined,
    runId: head.run_id,
    updatedAssertedAt: hlcToString(head.updated_asserted_at),
  };
}

function fromRustSchemaHead(head: RustSchemaHead): SchemaHead {
  return {
    partitionId: head.partition,
    typeId: head.type_id,
    schemaVersionHash: head.schema_version_hash,
    updatedAssertedAt: hlcToString(head.updated_asserted_at),
  };
}

function fromRustSchemaManifest(manifest: RustSchemaManifest): SchemaManifest {
  return {
    manifestVersion: manifest.manifest_version,
    migrations: manifest.migrations,
    tables: manifest.tables.map((table) => ({
      name: table.name,
      columns: table.columns.map((column) => ({
        name: column.name,
        logicalType: column.logical_type,
        nullable: column.nullable,
      })),
      indexes: table.indexes.map((index) => ({
        name: index.name,
        columns: index.columns,
        unique: index.unique,
      })),
    })),
  };
}

function fromRustExplainPrecedence(precedence: RustExplainPrecedence) {
  return {
    layer: precedence.layer,
    intervalWidth: precedence.interval_width,
    assertedAt: hlcToString(precedence.asserted_at),
    opId: precedence.op_id,
  };
}

function fromRustExplainPropertyFact(fact: RustExplainPropertyFact) {
  return {
    value: fromRustValue(fact.value),
    validFrom: fromValidTimeMicros(fact.valid_from) ?? '',
    validTo: fromValidTimeMicros(fact.valid_to),
    layer: fact.layer,
    assertedAt: hlcToString(fact.asserted_at),
    opId: fact.op_id,
    isTombstone: fact.is_tombstone,
    precedence: fromRustExplainPrecedence(fact.precedence),
  };
}

function fromRustExplainEdgeFact(fact: RustExplainEdgeFact) {
  return {
    edgeId: fact.edge_id,
    srcId: fact.src_id,
    dstId: fact.dst_id,
    edgeTypeId: fact.edge_type_id ?? undefined,
    validFrom: fromValidTimeMicros(fact.valid_from) ?? '',
    validTo: fromValidTimeMicros(fact.valid_to),
    layer: fact.layer,
    assertedAt: hlcToString(fact.asserted_at),
    opId: fact.op_id,
    isTombstone: fact.is_tombstone,
    precedence: fromRustExplainPrecedence(fact.precedence),
  };
}

function fromRustExplainResolution(result: RustExplainResolutionResult): ExplainResolutionResult {
  return {
    entityId: result.entity_id,
    fieldId: result.field_id,
    resolved: result.resolved ? toReadValue(result.resolved) : undefined,
    winner: result.winner ? fromRustExplainPropertyFact(result.winner) : undefined,
    candidates: result.candidates.map(fromRustExplainPropertyFact),
  };
}

function fromRustExplainTraversal(result: RustExplainTraversalResult): ExplainTraversalResult {
  return {
    edgeId: result.edge_id,
    active: result.active,
    winner: result.winner ? fromRustExplainEdgeFact(result.winner) : undefined,
    candidates: result.candidates.map(fromRustExplainEdgeFact),
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

const VALUE_TYPE_FROM_RUST: Record<RustValueType, EffectiveSchema['fields'][number]['valueType']> =
  {
    Str: 'str',
    I64: 'i64',
    F64: 'f64',
    Bool: 'bool',
    Time: 'time',
    Ref: 'ref',
    Blob: 'blob',
    Json: 'json',
  };

const MERGE_POLICY_FROM_RUST: Record<
  RustMergePolicy,
  EffectiveSchema['fields'][number]['mergePolicy']
> = {
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
