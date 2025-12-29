/**
 * Mneme DTOs and shared primitives for IPC bindings.
 */

export type Id = string;
export type PartitionId = string;
export type ActorId = string;
export type OpId = string;
export type EntityId = string;
export type TypeId = string;
export type FieldId = string;
export type ScenarioId = string;

export type ValidTime = string; // ISO-8601 UTC
export type AssertedTime = string; // HLC encoded as string

export type Layer = 'Plan' | 'Actual';

export type Value =
  | { t: 'str'; v: string }
  | { t: 'i64'; v: bigint }
  | { t: 'f64'; v: number }
  | { t: 'bool'; v: boolean }
  | { t: 'time'; v: ValidTime }
  | { t: 'ref'; v: EntityId }
  | { t: 'blob'; v: Uint8Array }
  | { t: 'json'; v: unknown };

export type ReadValue =
  | { k: 'single'; v: Value }
  | { k: 'multi'; v: Value[] }
  | { k: 'multi_limited'; v: { values: Value[]; moreAvailable: boolean } };

export interface TypeDef {
  typeId: TypeId;
  appliesTo: 'Node' | 'Edge';
  label: string;
  isAbstract: boolean;
  parentTypeId?: TypeId;
}

export interface FieldDef {
  fieldId: FieldId;
  label: string;
  valueType: 'str' | 'i64' | 'f64' | 'bool' | 'time' | 'ref' | 'blob' | 'json';
  cardinality: 'single' | 'multi';
  mergePolicy: 'LWW' | 'MV' | 'OR_SET' | 'COUNTER' | 'TEXT';
  indexed: boolean;
}

export interface TypeFieldDef {
  typeId: TypeId;
  fieldId: FieldId;
  required: boolean;
  defaultValue?: Value;
  overrideDefault?: boolean;
  tightenRequired?: boolean;
}

export interface EdgeTypeRuleDef {
  edgeTypeId: TypeId;
  semanticDirection: string;
  allowedSrcTypeIds?: TypeId[];
  allowedDstTypeIds?: TypeId[];
}

export interface MetamodelBatch {
  types: TypeDef[];
  fields: FieldDef[];
  typeFields: TypeFieldDef[];
  edgeTypeRules?: EdgeTypeRuleDef[];
}

export interface SchemaCompileResult {
  schemaVersionHash: string;
}
