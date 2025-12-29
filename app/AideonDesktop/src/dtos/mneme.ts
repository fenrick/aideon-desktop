/**
 * Mneme DTOs and shared primitives for IPC bindings.
 */

// ISO-8601 UTC
export type ValidTime = string;
// HLC encoded as string
export type AssertedTime = string;

export type Layer = 'Plan' | 'Actual';

export type Value =
  | { t: 'str'; v: string }
  | { t: 'i64'; v: bigint }
  | { t: 'f64'; v: number }
  | { t: 'bool'; v: boolean }
  | { t: 'time'; v: ValidTime }
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
  assertedAt: AssertedTime;
  nodeId: string;
  typeId?: string;
  scenarioId?: string;
}

export interface CreateEdgeInput {
  partitionId: string;
  actorId: string;
  assertedAt: AssertedTime;
  edgeId: string;
  typeId?: string;
  srcId: string;
  dstId: string;
  existsValidFrom: ValidTime;
  existsValidTo?: ValidTime;
  layer?: Layer;
  weight?: number;
  scenarioId?: string;
}

export interface SetEdgeExistenceIntervalInput {
  partitionId: string;
  actorId: string;
  assertedAt: AssertedTime;
  edgeId: string;
  validFrom: ValidTime;
  validTo?: ValidTime;
  layer?: Layer;
  isTombstone?: boolean;
  scenarioId?: string;
}

export interface TombstoneEntityInput {
  partitionId: string;
  actorId: string;
  assertedAt: AssertedTime;
  entityId: string;
  scenarioId?: string;
}

export interface SetPropertyIntervalInput {
  partitionId: string;
  actorId: string;
  assertedAt: AssertedTime;
  entityId: string;
  fieldId: string;
  value: Value;
  validFrom: ValidTime;
  validTo?: ValidTime;
  layer?: Layer;
  scenarioId?: string;
}

export interface ClearPropertyIntervalInput {
  partitionId: string;
  actorId: string;
  assertedAt: AssertedTime;
  entityId: string;
  fieldId: string;
  validFrom: ValidTime;
  validTo?: ValidTime;
  layer?: Layer;
  scenarioId?: string;
}

export interface OrSetUpdateInput {
  partitionId: string;
  actorId: string;
  assertedAt: AssertedTime;
  entityId: string;
  fieldId: string;
  op: 'Add' | 'Remove';
  element: Value;
  validFrom: ValidTime;
  validTo?: ValidTime;
  layer?: Layer;
  scenarioId?: string;
}

export interface CounterUpdateInput {
  partitionId: string;
  actorId: string;
  assertedAt: AssertedTime;
  entityId: string;
  fieldId: string;
  delta: number;
  validFrom: ValidTime;
  validTo?: ValidTime;
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
  at: ValidTime;
  asOfAssertedAt?: AssertedTime;
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
  at: ValidTime;
  asOfAssertedAt?: AssertedTime;
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
  at: ValidTime;
  asOfAssertedAt?: AssertedTime;
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
  at?: ValidTime;
  asOfAssertedAt?: AssertedTime;
  edgeTypeFilter?: string[];
  limit?: number;
  scenarioId?: string;
}

export interface GraphDegreeStat {
  entityId: string;
  outDegree: number;
  inDegree: number;
  asOfValidTime?: ValidTime;
  computedAssertedAt: AssertedTime;
}

export interface GetGraphDegreeStatsInput {
  partitionId: string;
  asOfValidTime?: ValidTime;
  entityIds?: string[];
  limit?: number;
  scenarioId?: string;
}

export interface GraphEdgeTypeCount {
  edgeTypeId?: string;
  count: number;
  computedAssertedAt: AssertedTime;
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

export interface PageRankRunParams {
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
  assertedAt: AssertedTime;
  asOfValidTime?: ValidTime;
  asOfAssertedAt?: AssertedTime;
  params: PageRankRunParams;
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
  assertedAt: AssertedTime;
  opType: number;
  payload: Uint8Array;
  deps: string[];
}

export interface ExportOpsInput {
  partitionId: string;
  sinceAssertedAt?: AssertedTime;
  limit?: number;
  scenarioId?: string;
}

export interface ExportOpsResult {
  ops: OpEnvelope[];
}

export interface IngestOpsInput {
  partitionId: string;
  ops: OpEnvelope[];
  scenarioId?: string;
}

export interface PartitionHeadResult {
  head: AssertedTime;
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
  createdAssertedAt: AssertedTime;
  updatedAssertedAt: AssertedTime;
  dedupeKey?: string;
  lastError?: string;
}

export interface ChangeEvent {
  partitionId: string;
  sequence: number;
  opId: string;
  assertedAt: AssertedTime;
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
  updatedAssertedAt: AssertedTime;
}

export interface SchemaHead {
  partitionId: string;
  typeId: string;
  schemaVersionHash: string;
  updatedAssertedAt: AssertedTime;
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
  assertedAt: AssertedTime;
  opId: string;
}

export interface ExplainPropertyFact {
  value: Value;
  validFrom: ValidTime;
  validTo?: ValidTime;
  layer: number;
  assertedAt: AssertedTime;
  opId: string;
  isTombstone: boolean;
  precedence: ExplainPrecedence;
}

export interface ExplainEdgeFact {
  edgeId: string;
  srcId: string;
  dstId: string;
  edgeTypeId?: string;
  validFrom: ValidTime;
  validTo?: ValidTime;
  layer: number;
  assertedAt: AssertedTime;
  opId: string;
  isTombstone: boolean;
  precedence: ExplainPrecedence;
}

export interface ExplainResolutionInput {
  partitionId: string;
  entityId: string;
  fieldId: string;
  at: ValidTime;
  asOfAssertedAt?: AssertedTime;
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
  at: ValidTime;
  asOfAssertedAt?: AssertedTime;
  scenarioId?: string;
}

export interface ExplainTraversalResult {
  edgeId: string;
  active: boolean;
  winner?: ExplainEdgeFact;
  candidates: ExplainEdgeFact[];
}
