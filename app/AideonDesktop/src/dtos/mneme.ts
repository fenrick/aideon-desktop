/**
 * Mneme DTOs and shared primitives for IPC bindings.
 */

export type Layer = 'Plan' | 'Actual';

export type Value =
  | { t: 'str'; v: string }
  | { t: 'i64'; v: bigint }
  | { t: 'f64'; v: number }
  | { t: 'bool'; v: boolean }
  | { t: 'time'; v: string }
  | { t: 'ref'; v: string }
  | { t: 'blob'; v: Uint8Array }
  | { t: 'json'; v: unknown };

export type ReadValue =
  | { k: 'single'; v: Value }
  | { k: 'multi'; v: Value[] }
  | { k: 'multi_limited'; v: { values: Value[]; moreAvailable: boolean } };

export interface TypeDefinition {
  typeId: string;
  appliesTo: 'Node' | 'Edge';
  label: string;
  isAbstract: boolean;
  parentTypeId?: string;
}

export interface FieldDefinition {
  fieldId: string;
  label: string;
  valueType: 'str' | 'i64' | 'f64' | 'bool' | 'time' | 'ref' | 'blob' | 'json';
  cardinality: 'single' | 'multi';
  mergePolicy: 'LWW' | 'MV' | 'OR_SET' | 'COUNTER' | 'TEXT';
  indexed: boolean;
}

export interface TypeFieldDefinition {
  typeId: string;
  fieldId: string;
  required: boolean;
  defaultValue?: Value;
  overrideDefault?: boolean;
  tightenRequired?: boolean;
}

export interface EdgeTypeRuleDefinition {
  edgeTypeId: string;
  semanticDirection: string;
  allowedSrcTypeIds?: string[];
  allowedDstTypeIds?: string[];
}

export interface MetamodelBatch {
  types: TypeDefinition[];
  fields: FieldDefinition[];
  typeFields: TypeFieldDefinition[];
  edgeTypeRules?: EdgeTypeRuleDefinition[];
}

export interface SchemaCompileResult {
  schemaVersionHash: string;
}

export interface CreateNodeInput {
  partitionId: string;
  actorId: string;
  assertedAt: string;
  nodeId: string;
  typeId?: string;
  scenarioId?: string;
}

export interface CreateEdgeInput {
  partitionId: string;
  actorId: string;
  assertedAt: string;
  edgeId: string;
  typeId?: string;
  srcId: string;
  dstId: string;
  existsValidFrom: string;
  existsValidTo?: string;
  layer?: Layer;
  weight?: number;
  scenarioId?: string;
}

export interface SetEdgeExistenceIntervalInput {
  partitionId: string;
  actorId: string;
  assertedAt: string;
  edgeId: string;
  validFrom: string;
  validTo?: string;
  layer?: Layer;
  isTombstone?: boolean;
  scenarioId?: string;
}

export interface TombstoneEntityInput {
  partitionId: string;
  actorId: string;
  assertedAt: string;
  entityId: string;
  scenarioId?: string;
}

export interface SetPropertyIntervalInput {
  partitionId: string;
  actorId: string;
  assertedAt: string;
  entityId: string;
  fieldId: string;
  value: Value;
  validFrom: string;
  validTo?: string;
  layer?: Layer;
  scenarioId?: string;
}

export interface ClearPropertyIntervalInput {
  partitionId: string;
  actorId: string;
  assertedAt: string;
  entityId: string;
  fieldId: string;
  validFrom: string;
  validTo?: string;
  layer?: Layer;
  scenarioId?: string;
}

export interface OrSetUpdateInput {
  partitionId: string;
  actorId: string;
  assertedAt: string;
  entityId: string;
  fieldId: string;
  op: 'Add' | 'Remove';
  element: Value;
  validFrom: string;
  validTo?: string;
  layer?: Layer;
  scenarioId?: string;
}

export interface CounterUpdateInput {
  partitionId: string;
  actorId: string;
  assertedAt: string;
  entityId: string;
  fieldId: string;
  delta: number;
  validFrom: string;
  validTo?: string;
  layer?: Layer;
  scenarioId?: string;
}

export type CompareOp = 'Eq' | 'Ne' | 'Lt' | 'Lte' | 'Gt' | 'Gte' | 'Prefix' | 'Contains';

export interface FieldFilter {
  fieldId: string;
  op: CompareOp;
  value: Value;
}

export interface ReadEntityAtTimeInput {
  partitionId: string;
  entityId: string;
  at: string;
  asOfAssertedAt?: string;
  fieldIds?: string[];
  includeDefaults?: boolean;
  scenarioId?: string;
}

export interface ReadEntityAtTimeResult {
  entityId: string;
  kind: 'Node' | 'Edge';
  typeId?: string;
  isDeleted: boolean;
  properties: Record<string, ReadValue>;
}

export type Direction = 'out' | 'in';

export interface TraverseAtTimeInput {
  partitionId: string;
  fromEntityId: string;
  direction: Direction;
  edgeTypeId?: string;
  at: string;
  asOfAssertedAt?: string;
  limit?: number;
  scenarioId?: string;
}

export interface TraverseEdgeItem {
  edgeId: string;
  srcId: string;
  dstId: string;
  edgeTypeId?: string;
}

export interface ListEntitiesInput {
  partitionId: string;
  at: string;
  asOfAssertedAt?: string;
  kind?: 'Node' | 'Edge';
  typeId?: string;
  filters?: FieldFilter[];
  limit?: number;
  cursor?: string;
  scenarioId?: string;
}

export interface ListEntitiesResultItem {
  entityId: string;
  kind: 'Node' | 'Edge';
  typeId?: string;
}

export interface ProjectionEdge {
  edgeId: string;
  srcId: string;
  dstId: string;
  edgeTypeId?: string;
  weight: number;
}

export interface GetProjectionEdgesInput {
  partitionId: string;
  at?: string;
  asOfAssertedAt?: string;
  edgeTypeFilter?: string[];
  limit?: number;
  scenarioId?: string;
}

export interface GraphDegreeStat {
  entityId: string;
  outDegree: number;
  inDegree: number;
  asOfValidTime?: string;
  computedAssertedAt: string;
}

export interface GetGraphDegreeStatsInput {
  partitionId: string;
  asOfValidTime?: string;
  entityIds?: string[];
  limit?: number;
  scenarioId?: string;
}

export interface GraphEdgeTypeCount {
  edgeTypeId?: string;
  count: number;
  computedAssertedAt: string;
}

export interface GetGraphEdgeTypeCountsInput {
  partitionId: string;
  edgeTypeIds?: string[];
  limit?: number;
  scenarioId?: string;
}

export interface PageRankSeed {
  id: string;
  w: number;
}

export interface PageRankRunParameters {
  damping: number;
  maxIters: number;
  tol: number;
  personalisedSeed?: PageRankSeed[];
}

export interface PageRankScore {
  id: string;
  score: number;
}

export interface StorePageRankRunInput {
  partitionId: string;
  actorId: string;
  assertedAt: string;
  asOfValidTime?: string;
  asOfAssertedAt?: string;
  params: PageRankRunParameters;
  scores: PageRankScore[];
  scenarioId?: string;
}

export interface PageRankRunResult {
  runId: string;
}

export interface GetPageRankScoresInput {
  partitionId: string;
  runId: string;
  topN: number;
  scenarioId?: string;
}

export interface OpEnvelope {
  opId: string;
  actorId: string;
  assertedAt: string;
  opType: number;
  payload: Uint8Array;
  deps: string[];
}

export interface ExportOpsInput {
  partitionId: string;
  sinceAssertedAt?: string;
  limit?: number;
  scenarioId?: string;
}

export interface ExportOpsResult {
  ops: OpEnvelope[];
}

export interface ExportOptions {
  partitionId: string;
  scenarioId?: string;
  sinceAssertedAt?: string;
  untilAssertedAt?: string;
  includeSchema?: boolean;
  includeDataOps?: boolean;
  includeScenarios?: boolean;
}

export interface ExportRecord {
  recordType: string;
  data: unknown;
}

export interface ImportOptions {
  targetPartition: string;
  scenarioId?: string;
  allowPartitionCreate?: boolean;
  remapActorIds?: Record<string, string>;
  strictSchema?: boolean;
}

export interface ImportReport {
  opsImported: number;
  opsSkipped: number;
  errors: number;
}

export interface IngestOpsInput {
  partitionId: string;
  ops: OpEnvelope[];
  scenarioId?: string;
}

export interface SnapshotOptions {
  partitionId: string;
  scenarioId?: string;
  asOfAssertedAt: string;
  includeFacts?: boolean;
  includeEntities?: boolean;
}

export interface ValidationRule {
  ruleId: string;
  scopeKind: number;
  scopeId?: string;
  severity: number;
  templateKind: string;
  params: unknown;
}

export interface UpsertValidationRulesInput {
  partitionId: string;
  actorId: string;
  assertedAt: string;
  rules: ValidationRule[];
}

export interface ComputedRule {
  ruleId: string;
  targetTypeId?: string;
  outputFieldId?: string;
  templateKind: string;
  params: unknown;
}

export interface UpsertComputedRulesInput {
  partitionId: string;
  actorId: string;
  assertedAt: string;
  rules: ComputedRule[];
}

export interface ComputedCacheEntry {
  entityId: string;
  fieldId: string;
  validFrom: string;
  validTo?: string;
  value: Value;
  ruleVersionHash: string;
  computedAssertedAt: string;
}

export interface UpsertComputedCacheInput {
  partitionId: string;
  entries: ComputedCacheEntry[];
}

export interface ListComputedCacheInput {
  partitionId: string;
  entityId?: string;
  fieldId: string;
  atValidTime?: string;
  limit?: number;
}

export interface PartitionHeadResult {
  head: string;
}

export interface GetPartitionHeadInput {
  partitionId: string;
  scenarioId?: string;
}

export interface CreateScenarioInput {
  partitionId: string;
  actorId: string;
  assertedAt: string;
  name: string;
}

export interface DeleteScenarioInput {
  partitionId: string;
  actorId: string;
  assertedAt: string;
  scenarioId: string;
}

export interface TriggerProcessingInput {
  partitionId: string;
  scenarioId?: string;
  reason: string;
}

export interface RetentionPolicy {
  keepOpsDays?: number;
  keepFactsDays?: number;
  keepFailedJobsDays?: number;
  keepPageRankRunsDays?: number;
}

export interface TriggerRetentionInput {
  partitionId: string;
  scenarioId?: string;
  policy: RetentionPolicy;
  reason: string;
}

export interface TriggerCompactionInput {
  partitionId: string;
  scenarioId?: string;
  reason: string;
}

export interface RunProcessingWorkerInput {
  maxJobs: number;
  leaseMillis: number;
}

export interface RunProcessingWorkerResult {
  jobsProcessed: number;
}

export interface ListJobsInput {
  partitionId: string;
  status?: number;
  limit: number;
}

export interface JobSummary {
  partitionId: string;
  jobId: string;
  jobType: string;
  status: number;
  priority: number;
  attempts: number;
  maxAttempts: number;
  leaseExpiresAt?: number;
  nextRunAfter?: number;
  createdAssertedAt: string;
  updatedAssertedAt: string;
  dedupeKey?: string;
  lastError?: string;
}

export interface ChangeEvent {
  partitionId: string;
  sequence: number;
  opId: string;
  assertedAt: string;
  entityId?: string;
  changeKind: number;
  payload?: unknown;
}

export interface GetChangesSinceInput {
  partitionId: string;
  fromSequence?: number;
  limit?: number;
}

export interface SubscribePartitionInput {
  partitionId: string;
  fromSequence?: number;
  eventName?: string;
}

export interface SubscriptionResult {
  subscriptionId: string;
}

export interface UnsubscribePartitionInput {
  subscriptionId: string;
}

export interface IntegrityHead {
  partitionId: string;
  scenarioId?: string;
  runId: string;
  updatedAssertedAt: string;
}

export interface SchemaHead {
  partitionId: string;
  typeId: string;
  schemaVersionHash: string;
  updatedAssertedAt: string;
}

export interface SchemaManifest {
  manifestVersion: string;
  migrations: string[];
  tables: TableManifest[];
}

export interface TableManifest {
  name: string;
  columns: ColumnManifest[];
  indexes: IndexManifest[];
}

export interface ColumnManifest {
  name: string;
  logicalType: string;
  nullable: boolean;
}

export interface IndexManifest {
  name: string;
  columns: string[];
  unique: boolean;
}

export interface ExplainPrecedence {
  layer: number;
  intervalWidth: number;
  assertedAt: string;
  opId: string;
}

export interface ExplainPropertyFact {
  value: Value;
  validFrom: string;
  validTo?: string;
  layer: number;
  assertedAt: string;
  opId: string;
  isTombstone: boolean;
  precedence: ExplainPrecedence;
}

export interface ExplainEdgeFact {
  edgeId: string;
  srcId: string;
  dstId: string;
  edgeTypeId?: string;
  validFrom: string;
  validTo?: string;
  layer: number;
  assertedAt: string;
  opId: string;
  isTombstone: boolean;
  precedence: ExplainPrecedence;
}

export interface ExplainResolutionInput {
  partitionId: string;
  entityId: string;
  fieldId: string;
  at: string;
  asOfAssertedAt?: string;
  scenarioId?: string;
}

export interface ExplainResolutionResult {
  entityId: string;
  fieldId: string;
  resolved?: ReadValue;
  winner?: ExplainPropertyFact;
  candidates: ExplainPropertyFact[];
}

export interface ExplainTraversalInput {
  partitionId: string;
  edgeId: string;
  at: string;
  asOfAssertedAt?: string;
  scenarioId?: string;
}

export interface ExplainTraversalResult {
  edgeId: string;
  active: boolean;
  winner?: ExplainEdgeFact;
  candidates: ExplainEdgeFact[];
}
