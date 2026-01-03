import { listen } from '@tauri-apps/api/event';
import type {
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
  GetPartitionHeadInput,
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
  ListJobsInput,
  MetamodelBatch,
  OpEnvelope,
  OrSetUpdateInput,
  PageRankRunParameters,
  PageRankRunResult,
  PageRankScore,
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
  Value,
} from 'dtos';

import { invokeIpc } from '../../adapters/ipc';
import { isTauri } from './platform';

const COMMANDS = {
  upsertMetamodelBatch: 'mneme.store.upsert_metamodel_batch',
  compileEffectiveSchema: 'mneme.store.compile_effective_schema',
  getEffectiveSchema: 'mneme.store.get_effective_schema',
  listEdgeTypeRules: 'mneme.store.list_edge_type_rules',
  createNode: 'mneme.store.create_node',
  createEdge: 'mneme.store.create_edge',
  setEdgeExistenceInterval: 'mneme.store.set_edge_existence_interval',
  tombstoneEntity: 'mneme.store.tombstone_entity',
  setPropertyInterval: 'mneme.store.set_property_interval',
  clearPropertyInterval: 'mneme.store.clear_property_interval',
  orSetUpdate: 'mneme.store.or_set_update',
  counterUpdate: 'mneme.store.counter_update',
  readEntityAtTime: 'mneme.store.read_entity_at_time',
  traverseAtTime: 'mneme.store.traverse_at_time',
  listEntities: 'mneme.store.list_entities',
  getProjectionEdges: 'mneme.store.get_projection_edges',
  getGraphDegreeStats: 'mneme.store.get_graph_degree_stats',
  getGraphEdgeTypeCounts: 'mneme.store.get_graph_edge_type_counts',
  storePageRankRun: 'mneme.store.store_pagerank_scores',
  getPageRankScores: 'mneme.store.get_pagerank_scores',
  exportOps: 'mneme.store.export_ops',
  ingestOps: 'mneme.store.ingest_ops',
  getPartitionHead: 'mneme.store.get_partition_head',
  createScenario: 'mneme.store.create_scenario',
  deleteScenario: 'mneme.store.delete_scenario',
  exportOpsStream: 'mneme.store.export_ops_stream',
  importOpsStream: 'mneme.store.import_ops_stream',
  exportSnapshotStream: 'mneme.store.export_snapshot_stream',
  importSnapshotStream: 'mneme.store.import_snapshot_stream',
  upsertValidationRules: 'mneme.store.upsert_validation_rules',
  listValidationRules: 'mneme.store.list_validation_rules',
  upsertComputedRules: 'mneme.store.upsert_computed_rules',
  listComputedRules: 'mneme.store.list_computed_rules',
  upsertComputedCache: 'mneme.store.upsert_computed_cache',
  listComputedCache: 'mneme.store.list_computed_cache',
  triggerRebuildEffectiveSchema: 'mneme.store.trigger_rebuild_effective_schema',
  triggerRefreshIntegrity: 'mneme.store.trigger_refresh_integrity',
  triggerRefreshAnalyticsProjections: 'mneme.store.trigger_refresh_analytics_projections',
  triggerRetention: 'mneme.store.trigger_retention',
  triggerCompaction: 'mneme.store.trigger_compaction',
  runProcessingWorker: 'mneme.store.run_processing_worker',
  listJobs: 'mneme.store.list_jobs',
  getChangesSince: 'mneme.store.get_changes_since',
  subscribePartition: 'mneme.store.subscribe_partition',
  unsubscribePartition: 'mneme.store.unsubscribe_partition',
  getIntegrityHead: 'mneme.store.get_integrity_head',
  getLastSchemaCompile: 'mneme.store.get_last_schema_compile',
  listFailedJobs: 'mneme.store.list_failed_jobs',
  getSchemaManifest: 'mneme.store.get_schema_manifest',
  explainResolution: 'mneme.store.explain_resolution',
  explainTraversal: 'mneme.store.explain_traversal',
} as const;

/**
 * Convert host/IPC errors into a readable message string.
 * @param error - Unknown thrown value.
 */
function formatIpcError(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === 'object' && error !== null) {
    const record = error as Record<string, unknown>;
    if (typeof record.message === 'string') {
      return record.message;
    }
  }
  return String(error);
}

/**
 * Invoke a host command using the request/response envelope contract.
 * @param command - Command name.
 * @param payload - Payload object.
 */
async function invokeHost<T>(command: string, payload: Record<string, unknown> = {}): Promise<T> {
  return invokeIpc<T>(command, payload);
}

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
  assertedAt: string;
  batch: MetamodelBatch;
  scenarioId?: string;
}

export interface CompileEffectiveSchemaInput {
  partitionId: string;
  actorId: string;
  assertedAt: string;
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
    return await invokeHost<MnemeOpResult>(COMMANDS.upsertMetamodelBatch, {
      partitionId: input.partitionId,
      actorId: input.actorId,
      assertedAt: input.assertedAt,
      scenarioId: input.scenarioId,
      batch: toRustMetamodelBatch(input.batch),
    });
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.upsertMetamodelBatch}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
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
    return await invokeHost<SchemaCompileResult>(
      COMMANDS.compileEffectiveSchema,
      toInvokeArguments(input),
    );
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.compileEffectiveSchema}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
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
    return await invokeHost<MnemeOpResult>(COMMANDS.createNode, toInvokeArguments(input));
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.createNode}' failed: ${formatIpcError(error)}`, {
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
    return await invokeHost<MnemeOpResult>(COMMANDS.createEdge, toInvokeArguments(input));
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.createEdge}' failed: ${formatIpcError(error)}`, {
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
    return await invokeHost<MnemeOpResult>(
      COMMANDS.setEdgeExistenceInterval,
      toInvokeArguments(input),
    );
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.setEdgeExistenceInterval}' failed: ${formatIpcError(error)}`,
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
    return await invokeHost<MnemeOpResult>(COMMANDS.tombstoneEntity, toInvokeArguments(input));
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.tombstoneEntity}' failed: ${formatIpcError(error)}`, {
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
    return await invokeHost<MnemeOpResult>(COMMANDS.setPropertyInterval, {
      ...input,
      value: toRustValue(input.value),
    });
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.setPropertyInterval}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
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
    return await invokeHost<MnemeOpResult>(
      COMMANDS.clearPropertyInterval,
      toInvokeArguments(input),
    );
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.clearPropertyInterval}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
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
    return await invokeHost<MnemeOpResult>(COMMANDS.orSetUpdate, {
      ...input,
      element: toRustValue(input.element),
    });
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.orSetUpdate}' failed: ${formatIpcError(error)}`, {
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
    return await invokeHost<MnemeOpResult>(COMMANDS.counterUpdate, toInvokeArguments(input));
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.counterUpdate}' failed: ${formatIpcError(error)}`, {
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
    const raw = await invokeHost<RustReadEntityAtTimeResult>(
      COMMANDS.readEntityAtTime,
      toInvokeArguments(input),
    );
    return fromRustReadEntityAtTime(raw);
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.readEntityAtTime}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
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
    const raw = await invokeHost<RustTraverseEdgeItem[]>(
      COMMANDS.traverseAtTime,
      toInvokeArguments(input),
    );
    return raw.map((edge) => ({
      edgeId: edge.edge_id,
      srcId: edge.src_id,
      dstId: edge.dst_id,
      edgeTypeId: edge.type_id ?? undefined,
    }));
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.traverseAtTime}' failed: ${formatIpcError(error)}`, {
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
    const raw = await invokeHost<RustListEntitiesResultItem[]>(COMMANDS.listEntities, {
      ...input,
      filters: (input.filters ?? []).map((item) => toRustFieldFilter(item)),
    });
    return raw.map((item) => ({
      entityId: item.entity_id,
      kind: item.kind,
      typeId: item.type_id ?? undefined,
    }));
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.listEntities}' failed: ${formatIpcError(error)}`, {
      cause: error,
    });
  }
}

/**
 * Fetch projection edges for analytics.
 * @param input
 */
export async function getProjectionEdges(
  input: GetProjectionEdgesInput,
): Promise<ProjectionEdge[]> {
  if (!isTauri()) {
    return [];
  }
  try {
    const raw = await invokeHost<RustProjectionEdge[]>(
      COMMANDS.getProjectionEdges,
      toInvokeArguments(input),
    );
    return raw.map((item) => fromRustProjectionEdge(item));
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.getProjectionEdges}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
  }
}

/**
 * Fetch graph degree stats.
 * @param input
 */
export async function getGraphDegreeStats(
  input: GetGraphDegreeStatsInput,
): Promise<GraphDegreeStat[]> {
  if (!isTauri()) {
    return [];
  }
  try {
    const raw = await invokeHost<RustGraphDegreeStat[]>(
      COMMANDS.getGraphDegreeStats,
      toInvokeArguments(input),
    );
    return raw.map((item) => fromRustGraphDegreeStat(item));
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.getGraphDegreeStats}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
  }
}

/**
 * Fetch edge type counts.
 * @param input
 */
export async function getGraphEdgeTypeCounts(
  input: GetGraphEdgeTypeCountsInput,
): Promise<GraphEdgeTypeCount[]> {
  if (!isTauri()) {
    return [];
  }
  try {
    const raw = await invokeHost<RustGraphEdgeTypeCount[]>(
      COMMANDS.getGraphEdgeTypeCounts,
      toInvokeArguments(input),
    );
    return raw.map((item) => fromRustGraphEdgeTypeCount(item));
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.getGraphEdgeTypeCounts}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
  }
}

/**
 * Store PageRank scores in Mneme.
 * @param input
 */
export async function storePageRankRun(input: StorePageRankRunInput): Promise<PageRankRunResult> {
  if (!isTauri()) {
    return { runId: 'mock-run' };
  }
  try {
    return await invokeHost<PageRankRunResult>(COMMANDS.storePageRankRun, {
      ...input,
      params: toRustPageRankParameters(input.params),
    });
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.storePageRankRun}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
  }
}

/**
 * Fetch PageRank scores.
 * @param input
 */
export async function getPageRankScores(input: GetPageRankScoresInput): Promise<PageRankScore[]> {
  if (!isTauri()) {
    return [];
  }
  try {
    return await invokeHost<PageRankScore[]>(COMMANDS.getPageRankScores, toInvokeArguments(input));
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.getPageRankScores}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
  }
}

/**
 * Export op log entries.
 * @param input
 */
export async function exportOps(input: ExportOpsInput): Promise<ExportOpsResult> {
  if (!isTauri()) {
    return { ops: [] };
  }
  try {
    const raw = await invokeHost<RustOpEnvelope[]>(COMMANDS.exportOps, toInvokeArguments(input));
    return { ops: raw.map((item) => fromRustOpEnvelope(item)) };
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.exportOps}' failed: ${formatIpcError(error)}`, {
      cause: error,
    });
  }
}

/**
 * Ingest op log entries.
 * @param input
 */
export async function ingestOps(input: IngestOpsInput): Promise<void> {
  if (!isTauri()) {
    return;
  }
  try {
    await invokeHost(COMMANDS.ingestOps, {
      ...input,
      ops: input.ops.map((item) => toRustOpEnvelope(item)),
    });
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.ingestOps}' failed: ${formatIpcError(error)}`, {
      cause: error,
    });
  }
}

/**
 * Fetch the partition head.
 * @param input
 */
export async function getPartitionHead(input: GetPartitionHeadInput): Promise<PartitionHeadResult> {
  if (!isTauri()) {
    return { head: '0' };
  }
  try {
    return await invokeHost<PartitionHeadResult>(
      COMMANDS.getPartitionHead,
      toInvokeArguments(input),
    );
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.getPartitionHead}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
  }
}

/**
 * Create a scenario overlay.
 * @param input
 */
export async function createScenario(input: CreateScenarioInput): Promise<string> {
  if (!isTauri()) {
    return 'mock-scenario';
  }
  try {
    return await invokeHost<string>(COMMANDS.createScenario, toInvokeArguments(input));
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.createScenario}' failed: ${formatIpcError(error)}`, {
      cause: error,
    });
  }
}

/**
 * Delete a scenario overlay.
 * @param input
 */
export async function deleteScenario(input: DeleteScenarioInput): Promise<void> {
  if (!isTauri()) {
    return;
  }
  try {
    await invokeHost(COMMANDS.deleteScenario, toInvokeArguments(input));
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.deleteScenario}' failed: ${formatIpcError(error)}`, {
      cause: error,
    });
  }
}

/**
 * Export a streaming op log payload.
 * @param options
 */
export async function exportOpsStream(
  options: ExportOptions,
): Promise<AsyncIterable<ExportRecord>> {
  if (!isTauri()) {
    return toAsyncIterable([]);
  }
  try {
    const raw = await invokeHost<RustExportRecord[]>(
      COMMANDS.exportOpsStream,
      toInvokeArguments(options),
    );
    return toAsyncIterable(raw.map((item) => fromRustExportRecord(item)));
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.exportOpsStream}' failed: ${formatIpcError(error)}`, {
      cause: error,
    });
  }
}

/**
 * Import a streaming op log payload.
 * @param options
 * @param records
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
    const raw = await invokeHost<RustImportReport>(COMMANDS.importOpsStream, {
      ...options,
      records: collected.map((item) => toRustExportRecord(item)),
    });
    return fromRustImportReport(raw);
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.importOpsStream}' failed: ${formatIpcError(error)}`, {
      cause: error,
    });
  }
}

/**
 * Export a snapshot stream.
 * @param options
 */
export async function exportSnapshotStream(
  options: SnapshotOptions,
): Promise<AsyncIterable<ExportRecord>> {
  if (!isTauri()) {
    return toAsyncIterable([]);
  }
  try {
    const raw = await invokeHost<RustExportRecord[]>(
      COMMANDS.exportSnapshotStream,
      toInvokeArguments(options),
    );
    return toAsyncIterable(raw.map((item) => fromRustExportRecord(item)));
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.exportSnapshotStream}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
  }
}

/**
 * Import a snapshot stream.
 * @param options
 * @param records
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
    await invokeHost(COMMANDS.importSnapshotStream, {
      ...options,
      records: collected.map((item) => toRustExportRecord(item)),
    });
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.importSnapshotStream}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
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
 * @param input
 */
export async function upsertValidationRules(input: UpsertValidationRulesInput): Promise<void> {
  if (!isTauri()) {
    return;
  }
  try {
    await invokeHost(COMMANDS.upsertValidationRules, {
      ...input,
      rules: input.rules.map((item) => toRustValidationRule(item)),
    });
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.upsertValidationRules}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
  }
}

/**
 * List validation rules for a partition.
 * @param partitionId
 */
export async function listValidationRules(partitionId: string): Promise<ValidationRule[]> {
  if (!isTauri()) {
    return [];
  }
  try {
    const raw = await invokeHost<RustValidationRule[]>(COMMANDS.listValidationRules, {
      partitionId,
    });
    return raw.map((item) => fromRustValidationRule(item));
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.listValidationRules}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
  }
}

/**
 * Upsert computed rules for a partition.
 * @param input
 */
export async function upsertComputedRules(input: UpsertComputedRulesInput): Promise<void> {
  if (!isTauri()) {
    return;
  }
  try {
    await invokeHost(COMMANDS.upsertComputedRules, {
      ...input,
      rules: input.rules.map((item) => toRustComputedRule(item)),
    });
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.upsertComputedRules}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
  }
}

/**
 * List computed rules for a partition.
 * @param partitionId
 */
export async function listComputedRules(partitionId: string): Promise<ComputedRule[]> {
  if (!isTauri()) {
    return [];
  }
  try {
    const raw = await invokeHost<RustComputedRule[]>(COMMANDS.listComputedRules, { partitionId });
    return raw.map((item) => fromRustComputedRule(item));
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.listComputedRules}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
  }
}

/**
 * Upsert computed cache entries for a partition.
 * @param input
 */
export async function upsertComputedCache(input: UpsertComputedCacheInput): Promise<void> {
  if (!isTauri()) {
    return;
  }
  try {
    await invokeHost(COMMANDS.upsertComputedCache, {
      partitionId: input.partitionId,
      entries: input.entries.map((item) => toRustComputedCacheEntry(item)),
    });
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.upsertComputedCache}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
  }
}

/**
 * List computed cache entries.
 * @param input
 */
export async function listComputedCache(
  input: ListComputedCacheInput,
): Promise<ComputedCacheEntry[]> {
  if (!isTauri()) {
    return [];
  }
  try {
    const raw = await invokeHost<RustComputedCacheEntry[]>(
      COMMANDS.listComputedCache,
      toInvokeArguments(input),
    );
    return raw.map((item) => fromRustComputedCacheEntry(item));
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.listComputedCache}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
  }
}

/**
 *
 * @param input
 */
export async function triggerRebuildEffectiveSchema(input: TriggerProcessingInput): Promise<void> {
  if (!isTauri()) {
    return;
  }
  try {
    await invokeHost(COMMANDS.triggerRebuildEffectiveSchema, toInvokeArguments(input));
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.triggerRebuildEffectiveSchema}' failed: ${formatIpcError(error)}`,
      { cause: error },
    );
  }
}

/**
 *
 * @param input
 */
export async function triggerRefreshIntegrity(input: TriggerProcessingInput): Promise<void> {
  if (!isTauri()) {
    return;
  }
  try {
    await invokeHost(COMMANDS.triggerRefreshIntegrity, toInvokeArguments(input));
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.triggerRefreshIntegrity}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
  }
}

/**
 *
 * @param input
 */
export async function triggerRefreshAnalyticsProjections(
  input: TriggerProcessingInput,
): Promise<void> {
  if (!isTauri()) {
    return;
  }
  try {
    await invokeHost(COMMANDS.triggerRefreshAnalyticsProjections, toInvokeArguments(input));
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.triggerRefreshAnalyticsProjections}' failed: ${formatIpcError(error)}`,
      { cause: error },
    );
  }
}

/**
 *
 * @param input
 */
export async function triggerRetention(input: TriggerRetentionInput): Promise<void> {
  if (!isTauri()) {
    return;
  }
  try {
    await invokeHost(COMMANDS.triggerRetention, toInvokeArguments(input));
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.triggerRetention}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
  }
}

/**
 *
 * @param input
 */
export async function triggerCompaction(input: TriggerCompactionInput): Promise<void> {
  if (!isTauri()) {
    return;
  }
  try {
    await invokeHost(COMMANDS.triggerCompaction, toInvokeArguments(input));
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.triggerCompaction}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
  }
}

/**
 *
 * @param input
 */
export async function runProcessingWorker(
  input: RunProcessingWorkerInput,
): Promise<RunProcessingWorkerResult> {
  if (!isTauri()) {
    return { jobsProcessed: 0 };
  }
  try {
    return await invokeHost<RunProcessingWorkerResult>(
      COMMANDS.runProcessingWorker,
      toInvokeArguments(input),
    );
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.runProcessingWorker}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
  }
}

/**
 *
 * @param input
 */
export async function listJobs(input: ListJobsInput): Promise<JobSummary[]> {
  if (!isTauri()) {
    return [];
  }
  try {
    const raw = await invokeHost<RustJobSummary[]>(COMMANDS.listJobs, toInvokeArguments(input));
    return raw.map((item) => fromRustJobSummary(item));
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.listJobs}' failed: ${formatIpcError(error)}`, {
      cause: error,
    });
  }
}

/**
 *
 * @param input
 */
export async function getChangesSince(input: GetChangesSinceInput): Promise<ChangeEvent[]> {
  if (!isTauri()) {
    return [];
  }
  try {
    const raw = await invokeHost<RustChangeEvent[]>(
      COMMANDS.getChangesSince,
      toInvokeArguments(input),
    );
    return raw.map((item) => fromRustChangeEvent(item));
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.getChangesSince}' failed: ${formatIpcError(error)}`, {
      cause: error,
    });
  }
}

/**
 *
 * @param input
 */
export async function subscribePartition(
  input: SubscribePartitionInput,
): Promise<SubscriptionResult> {
  if (!isTauri()) {
    return { subscriptionId: 'mock-sub' };
  }
  try {
    return await invokeHost<SubscriptionResult>(
      COMMANDS.subscribePartition,
      toInvokeArguments(input),
    );
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.subscribePartition}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
  }
}

/**
 *
 * @param input
 */
export async function unsubscribePartition(input: UnsubscribePartitionInput): Promise<boolean> {
  if (!isTauri()) {
    return true;
  }
  try {
    return await invokeHost<boolean>(COMMANDS.unsubscribePartition, toInvokeArguments(input));
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.unsubscribePartition}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
  }
}

/**
 *
 * @param handler
 */
export async function onChangeEvents(handler: (event: ChangeEvent) => void): Promise<() => void> {
  if (!isTauri()) {
    return () => {
      return;
    };
  }
  const unlisten = await listen<RustChangeEvent>('mneme_change_event', (event) => {
    handler(fromRustChangeEvent(event.payload));
  });
  return unlisten;
}

/**
 *
 * @param partitionId
 * @param scenarioId
 */
export async function getIntegrityHead(
  partitionId: string,
  scenarioId?: string,
): Promise<IntegrityHead | undefined> {
  if (!isTauri()) {
    return undefined;
  }
  try {
    const raw = await invokeHost<RustIntegrityHead | null>(COMMANDS.getIntegrityHead, {
      partitionId,
      scenarioId,
    });
    return raw ? fromRustIntegrityHead(raw) : undefined;
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.getIntegrityHead}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
  }
}

/**
 *
 * @param partitionId
 * @param typeId
 */
export async function getLastSchemaCompile(
  partitionId: string,
  typeId: string,
): Promise<SchemaHead | undefined> {
  if (!isTauri()) {
    return undefined;
  }
  try {
    const raw = await invokeHost<RustSchemaHead | null>(COMMANDS.getLastSchemaCompile, {
      partitionId,
      typeId,
    });
    return raw ? fromRustSchemaHead(raw) : undefined;
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.getLastSchemaCompile}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
  }
}

/**
 *
 * @param partitionId
 * @param limit
 */
export async function listFailedJobs(partitionId: string, limit: number): Promise<JobSummary[]> {
  if (!isTauri()) {
    return [];
  }
  try {
    const raw = await invokeHost<RustJobSummary[]>(COMMANDS.listFailedJobs, {
      partitionId,
      limit,
    });
    return raw.map((item) => fromRustJobSummary(item));
  } catch (error) {
    throw new Error(`Host command '${COMMANDS.listFailedJobs}' failed: ${formatIpcError(error)}`, {
      cause: error,
    });
  }
}

/**
 *
 */
export async function getSchemaManifest(): Promise<SchemaManifest> {
  if (!isTauri()) {
    return { manifestVersion: '0', migrations: [], tables: [] };
  }
  try {
    const raw = await invokeHost<RustSchemaManifest>(COMMANDS.getSchemaManifest);
    return fromRustSchemaManifest(raw);
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.getSchemaManifest}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
  }
}

/**
 *
 * @param input
 */
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
    const raw = await invokeHost<RustExplainResolutionResult>(
      COMMANDS.explainResolution,
      toInvokeArguments(input),
    );
    return fromRustExplainResolution(raw);
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.explainResolution}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
  }
}

/**
 *
 * @param input
 */
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
    const raw = await invokeHost<RustExplainTraversalResult>(
      COMMANDS.explainTraversal,
      toInvokeArguments(input),
    );
    return fromRustExplainTraversal(raw);
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.explainTraversal}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
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
    const raw = await invokeHost<RustEffectiveSchema | null>(COMMANDS.getEffectiveSchema, {
      partitionId,
      typeId,
    });
    return raw ? fromRustEffectiveSchema(raw) : undefined;
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.getEffectiveSchema}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
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
    const raw = await invokeHost<RustEdgeTypeRule[]>(COMMANDS.listEdgeTypeRules, {
      partitionId,
      edgeTypeId,
    });
    return raw.map((rule) => fromRustEdgeTypeRule(rule));
  } catch (error) {
    throw new Error(
      `Host command '${COMMANDS.listEdgeTypeRules}' failed: ${formatIpcError(error)}`,
      {
        cause: error,
      },
    );
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

interface RustPageRankParameters {
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
): RustValueType {
  if (valueType in VALUE_TYPE_MAP) {
    return VALUE_TYPE_MAP[valueType as EffectiveSchema['fields'][number]['valueType']];
  }
  return valueType as RustValueType;
}

/**
 * Normalize merge policies to the Rust enum representation.
 * @param mergePolicy - Merge policy in either Rust or API form.
 */
function toRustMergePolicy(
  mergePolicy: RustMergePolicy | EffectiveSchema['fields'][number]['mergePolicy'],
): RustMergePolicy {
  if (mergePolicy in MERGE_POLICY_MAP) {
    return MERGE_POLICY_MAP[mergePolicy as EffectiveSchema['fields'][number]['mergePolicy']];
  }
  return mergePolicy as RustMergePolicy;
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
    types: batch.types.map((typeDefinition) => toRustType(typeDefinition)),
    fields: batch.fields.map((fieldDefinition) => toRustField(fieldDefinition)),
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
function toValidTimeMicros(value: string): number {
  const parsed = Date.parse(value);
  if (Number.isNaN(parsed)) {
    throw new TypeError(`Invalid ISO-8601 timestamp: ${value}`);
  }
  return parsed * 1000;
}

/**
 *
 * @param value
 */
function fromValidTimeMicros(value?: number | null): string | undefined {
  if (value === undefined || value === null) {
    return undefined;
  }
  return new Date(value / 1000).toISOString();
}

/**
 *
 * @param parameters
 */
function toRustPageRankParameters(parameters: PageRankRunParameters): RustPageRankParameters {
  return {
    ...parameters,
    personalisedSeed: parameters.personalisedSeed?.map((seed) => ({
      id: seed.id,
      weight: seed.w,
    })),
  };
}

/**
 *
 * @param payload
 */
function normalizeBytes(payload: Uint8Array | number[]): Uint8Array {
  if (payload instanceof Uint8Array) {
    return payload;
  }
  return Uint8Array.from(payload);
}

/**
 *
 * @param op
 */
function fromRustOpEnvelope(op: RustOpEnvelope): OpEnvelope {
  return {
    opId: op.op_id,
    actorId: op.actor_id,
    assertedAt: String(op.asserted_at),
    opType: op.op_type,
    payload: normalizeBytes(op.payload),
    deps: op.deps,
  };
}

/**
 *
 * @param op
 */
function toRustOpEnvelope(op: OpEnvelope) {
  return {
    opId: op.opId,
    actorId: op.actorId,
    assertedAt: op.assertedAt,
    opType: op.opType,
    payload: [...op.payload],
    deps: op.deps,
  };
}

/**
 *
 * @param record
 */
function fromRustExportRecord(record: RustExportRecord): ExportRecord {
  return {
    recordType: record.record_type,
    data: record.data,
  };
}

/**
 *
 * @param record
 */
function toRustExportRecord(record: ExportRecord): RustExportRecord {
  return {
    record_type: record.recordType,
    data: record.data,
  };
}

/**
 *
 * @param report
 */
function fromRustImportReport(report: RustImportReport): ImportReport {
  return {
    opsImported: report.ops_imported,
    opsSkipped: report.ops_skipped,
    errors: report.errors,
  };
}

/**
 *
 * @param rule
 */
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

/**
 *
 * @param rule
 */
function toRustValidationRule(rule: ValidationRule): RustValidationRule {
  return {
    rule_id: rule.ruleId,
    scope_kind: rule.scopeKind,
    scope_id: rule.scopeId ?? undefined,
    severity: rule.severity,
    template_kind: rule.templateKind,
    params: rule.params,
  };
}

/**
 *
 * @param rule
 */
function fromRustComputedRule(rule: RustComputedRule): ComputedRule {
  return {
    ruleId: rule.rule_id,
    targetTypeId: rule.target_type_id ?? undefined,
    outputFieldId: rule.output_field_id ?? undefined,
    templateKind: rule.template_kind,
    params: rule.params,
  };
}

/**
 *
 * @param rule
 */
function toRustComputedRule(rule: ComputedRule): RustComputedRule {
  return {
    rule_id: rule.ruleId,
    target_type_id: rule.targetTypeId ?? undefined,
    output_field_id: rule.outputFieldId ?? undefined,
    template_kind: rule.templateKind,
    params: rule.params,
  };
}

/**
 *
 * @param entry
 */
function fromRustComputedCacheEntry(entry: RustComputedCacheEntry): ComputedCacheEntry {
  return {
    entityId: entry.entity_id,
    fieldId: entry.field_id,
    validFrom: fromValidTimeMicros(entry.valid_from) ?? '',
    validTo: fromValidTimeMicros(entry.valid_to),
    value: fromRustValue(entry.value),
    ruleVersionHash: entry.rule_version_hash,
    computedAssertedAt: String(entry.computed_asserted_at),
  };
}

/**
 *
 * @param entry
 */
function toRustComputedCacheEntry(entry: ComputedCacheEntry): RustComputedCacheEntryPayload {
  return {
    entity_id: entry.entityId,
    field_id: entry.fieldId,
    valid_from: entry.validFrom,
    valid_to: entry.validTo ?? undefined,
    value: toRustValue(entry.value),
    rule_version_hash: entry.ruleVersionHash,
    computed_asserted_at: entry.computedAssertedAt,
  };
}

/**
 *
 * @param items
 * @yields {T} item from the input list
 */
async function* toAsyncIterable<T>(items: T[]): AsyncIterable<T> {
  for (const item of items) {
    await Promise.resolve();
    yield item;
  }
}

/**
 *
 * @param items
 */
async function collectAsyncIterable<T>(items: AsyncIterable<T>): Promise<T[]> {
  const collected: T[] = [];
  for await (const item of items) {
    collected.push(item);
  }
  return collected;
}

/**
 *
 * @param job
 */
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
    createdAssertedAt: String(job.created_asserted_at),
    updatedAssertedAt: String(job.updated_asserted_at),
    dedupeKey: job.dedupe_key ?? undefined,
    lastError: job.last_error ?? undefined,
  };
}

/**
 *
 * @param event
 */
function fromRustChangeEvent(event: RustChangeEvent): ChangeEvent {
  return {
    partitionId: event.partition,
    sequence: event.sequence,
    opId: event.op_id,
    assertedAt: String(event.asserted_at),
    entityId: event.entity_id ?? undefined,
    changeKind: event.change_kind,
    payload: event.payload,
  };
}

/**
 *
 * @param head
 */
function fromRustIntegrityHead(head: RustIntegrityHead): IntegrityHead {
  return {
    partitionId: head.partition,
    scenarioId: head.scenario_id ?? undefined,
    runId: head.run_id,
    updatedAssertedAt: String(head.updated_asserted_at),
  };
}

/**
 *
 * @param head
 */
function fromRustSchemaHead(head: RustSchemaHead): SchemaHead {
  return {
    partitionId: head.partition,
    typeId: head.type_id,
    schemaVersionHash: head.schema_version_hash,
    updatedAssertedAt: String(head.updated_asserted_at),
  };
}

/**
 *
 * @param manifest
 */
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

/**
 *
 * @param precedence
 */
function fromRustExplainPrecedence(precedence: RustExplainPrecedence) {
  return {
    layer: precedence.layer,
    intervalWidth: precedence.interval_width,
    assertedAt: String(precedence.asserted_at),
    opId: precedence.op_id,
  };
}

/**
 *
 * @param fact
 */
function fromRustExplainPropertyFact(fact: RustExplainPropertyFact) {
  return {
    value: fromRustValue(fact.value),
    validFrom: fromValidTimeMicros(fact.valid_from) ?? '',
    validTo: fromValidTimeMicros(fact.valid_to),
    layer: fact.layer,
    assertedAt: String(fact.asserted_at),
    opId: fact.op_id,
    isTombstone: fact.is_tombstone,
    precedence: fromRustExplainPrecedence(fact.precedence),
  };
}

/**
 *
 * @param fact
 */
function fromRustExplainEdgeFact(fact: RustExplainEdgeFact) {
  return {
    edgeId: fact.edge_id,
    srcId: fact.src_id,
    dstId: fact.dst_id,
    edgeTypeId: fact.edge_type_id ?? undefined,
    validFrom: fromValidTimeMicros(fact.valid_from) ?? '',
    validTo: fromValidTimeMicros(fact.valid_to),
    layer: fact.layer,
    assertedAt: String(fact.asserted_at),
    opId: fact.op_id,
    isTombstone: fact.is_tombstone,
    precedence: fromRustExplainPrecedence(fact.precedence),
  };
}

/**
 *
 * @param result
 */
function fromRustExplainResolution(result: RustExplainResolutionResult): ExplainResolutionResult {
  return {
    entityId: result.entity_id,
    fieldId: result.field_id,
    resolved: result.resolved ? toReadValue(result.resolved) : undefined,
    winner: result.winner ? fromRustExplainPropertyFact(result.winner) : undefined,
    candidates: result.candidates.map((item) => fromRustExplainPropertyFact(item)),
  };
}

/**
 *
 * @param result
 */
function fromRustExplainTraversal(result: RustExplainTraversalResult): ExplainTraversalResult {
  return {
    edgeId: result.edge_id,
    active: result.active,
    winner: result.winner ? fromRustExplainEdgeFact(result.winner) : undefined,
    candidates: result.candidates.map((item) => fromRustExplainEdgeFact(item)),
  };
}

/**
 * Convert a Rust value into the API representation.
 * @param value - Rust value to convert.
 */
function fromRustValue(value: RustValue): Value {
  const [kind, payload] = Object.entries(value)[0] ?? [];
  if (!kind) {
    throw new TypeError('Unsupported Rust value variant');
  }
  switch (kind) {
    case 'Str': {
      return { t: 'str', v: payload as string };
    }
    case 'I64': {
      return { t: 'i64', v: BigInt(payload as number) };
    }
    case 'F64': {
      return { t: 'f64', v: payload as number };
    }
    case 'Bool': {
      return { t: 'bool', v: payload as boolean };
    }
    case 'Time': {
      return { t: 'time', v: new Date((payload as number) / 1000).toISOString() };
    }
    case 'Ref': {
      return { t: 'ref', v: payload as string };
    }
    case 'Blob': {
      const blobPayload = payload as Uint8Array | number[];
      const blob = blobPayload instanceof Uint8Array ? blobPayload : Uint8Array.from(blobPayload);
      return { t: 'blob', v: blob };
    }
    case 'Json': {
      return { t: 'json', v: payload };
    }
    default: {
      throw new TypeError('Unsupported Rust value variant');
    }
  }
}

/**
 * Convert a Rust read result into the API representation.
 * @param raw - Rust payload.
 */
function fromRustReadEntityAtTime(raw: RustReadEntityAtTimeResult): ReadEntityAtTimeResult {
  const properties = Object.fromEntries(
    Object.entries(raw.properties).map(([fieldId, value]) => [fieldId, toReadValue(value)]),
  ) as Record<string, ReadValue>;
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

/**
 *
 * @param edge
 */
function fromRustProjectionEdge(edge: RustProjectionEdge): ProjectionEdge {
  return {
    edgeId: edge.edge_id,
    srcId: edge.src_id,
    dstId: edge.dst_id,
    edgeTypeId: edge.edge_type_id ?? undefined,
    weight: edge.weight,
  };
}

/**
 *
 * @param stat
 */
function fromRustGraphDegreeStat(stat: RustGraphDegreeStat): GraphDegreeStat {
  return {
    entityId: stat.entity_id,
    outDegree: stat.out_degree,
    inDegree: stat.in_degree,
    asOfValidTime: fromValidTimeMicros(stat.as_of_valid_time),
    computedAssertedAt: String(stat.computed_asserted_at),
  };
}

/**
 *
 * @param count
 */
function fromRustGraphEdgeTypeCount(count: RustGraphEdgeTypeCount): GraphEdgeTypeCount {
  return {
    edgeTypeId: count.edge_type_id ?? undefined,
    count: count.count,
    computedAssertedAt: String(count.computed_asserted_at),
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
