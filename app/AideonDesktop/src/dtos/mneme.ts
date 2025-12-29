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
