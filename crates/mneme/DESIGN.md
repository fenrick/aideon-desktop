# Aideon Mneme Storage Engine Spec (SeaORM + SQLx, RDBMS-abstract)

## 1. Purpose and boundaries

### 1.1 Mneme responsibilities

Mneme is the system’s **single storage engine** for all EA repository data and related analytics projections. It provides:

- A strict **Rust API** and **TypeScript API** for reading and writing EA graph data.
- A **SeaORM/SeaQuery migration engine** that creates and evolves the database schema across supported SQLx backends without raw SQL.
- A **bi-temporal fact store** supporting:
  - valid-time playback (past/future)
  - “as-of asserted time” audit
  - future plateaus and backdated corrections

- A **directed property-graph storage model**:
  - nodes and edges as first-class entities
  - edges have endpoints, properties, and time-valid existence
  - fast adjacency traversal with indexes

- A **projection layer** that exposes clean directed adjacency for analytics (including PageRank-style algorithms) without other layers generating SQL.
- A **sync-ready operation log** (append-only), with idempotent ingestion and deterministic resolution rules.

### 1.2 What Mneme does not do

- It does not define the default EA metamodel. Praxis (or another module) provides the metamodel via Mneme’s metamodel API.
- It does not expose arbitrary SQL execution or query builder primitives to other modules.
- It does not require Git metaphors; scenario/what-if is implemented via explicit scenario primitives.

---

## 2. Supported database technologies (SeaORM + SQLx constraints)

Mneme targets only databases supported by **SeaORM + SQLx** without raw SQL:

- PostgreSQL
- MySQL / MariaDB
- SQLite

All schema, migrations, indexes, and constraints must be expressible through:

- `sea_orm_migration` + `sea_query`
- SeaORM entities and query builders

No raw SQL strings in migrations or runtime paths.

---

## 3. Core concepts and invariants

### 3.1 Partitioning

All data is scoped by a mandatory `partition_id` (workspace/tenant). This:

- bounds query and sync working sets
- prevents cross-tenant leakage
- enables independent scenario models per partition

### 3.2 Identity and timestamps

- `Id`: 128-bit identifier (UUID).
  - DB representation:
    - Postgres: `Uuid`
    - MySQL: `Binary(16)` or `Char(36)` (prefer `Binary(16)` if supported in SeaORM schema)
    - SQLite: `Text` (UUID string)

  - Mneme uses a unified encoding/decoding layer and exposes IDs as strings in TS.

- `asserted_at`: **HLC** (hybrid logical clock) encoded as `i64` or `i128` canonical form.
  - Cross-db friendly representation: `BigInteger` (`i64` if sufficient; `i128` if required—SeaORM portability favours `i64`).
  - This is assertion-time.

- `valid_from`, `valid_to`: valid-time interval bounds.
  - Represent as `i64` epoch microseconds UTC.
  - `valid_to` nullable; semantics: `[valid_from, valid_to)` or `[valid_from, +∞)`.

### 3.3 Bi-temporal semantics

Every fact that affects state at time T stores:

- valid interval (`valid_from`, `valid_to`)
- asserted time (`asserted_at`)
- provenance (`op_id`, `actor_id`)
- precedence (`layer`)

Two query modes:

- Playback: “truth as known now” (no asserted-time filter)
- Audit: “truth as-of asserted time A” (filter `asserted_at <= A`)

### 3.4 Deterministic resolution

At a given valid time `T`, the winning fact(s) for `(entity, field)` are selected by:

1. valid containment: `valid_from <= T AND (valid_to IS NULL OR T < valid_to)`
2. layer precedence: higher layer wins (e.g., Actual > Plan)
3. interval specificity: narrower interval wins (smaller duration; `NULL valid_to` treated as widest)
4. stable tie-break: higher `asserted_at`, then `op_id` (lexicographic/bytes) to be deterministic

Merge policy then applies:

- `LWW`: single winner
- `MV`: multiple winners (bounded)
- `OR_SET`: set semantics (requires add/remove element facts)
- `COUNTER`: sum deltas
- `TEXT`: out of scope for v1 storage; supported as `MV` or `LWW` on string unless a separate text engine is introduced

### 3.5 Graph analytics readiness (PageRank)

Mneme must provide a **clean directed adjacency projection** so analytics algorithms operate on consistent directionality and edge typing. This is enforced by:

- schema storage for edge types and relationship semantics (direction labels)
- runtime projection tables updated transactionally
- APIs to fetch adjacency at a valid-time slice

---

## 4. Data model: separation of storage vs metamodel definition

Mneme stores metamodel as data, but it does not prescribe the default metamodel. Praxis defines and submits:

- types (node/edge classes)
- fields (property definitions)
- inheritance (single parent)
- field attachments (constraints + default values)
- edge-type endpoint constraints (optional) and semantic direction labels (recommended)

Mneme uses that to:

- validate writes (configurable strictness)
- compute effective schema caches (derived)
- resolve defaults during reads

---

## 5. Physical schema (tables and indexes)

All tables are prefixed with `aideon_`.

### 5.1 Common columns and conventions

- All primary keys include `partition_id` unless globally unique identifiers are guaranteed and cross-partition queries never happen. Mneme requires partition-scoped uniqueness and consistent query planning, so `partition_id` is part of most PKs.
- Where DB-specific types vary (UUID), Mneme uses SeaORM’s portable representations and conversion helpers.

---

## 6. Tables (authoritative)

### 6.1 Partitions and actors

#### `aideon_partitions`

- `partition_id` (PK)
- `created_asserted_at` (i64)
- `created_by_actor_id` (nullable)

Indexes:

- none additional

#### `aideon_actors`

- `partition_id` (PK part)
- `actor_id` (PK part)
- `metadata_json` (nullable, text/json)
- `created_asserted_at` (i64)

Indexes:

- `(partition_id, actor_id)` is PK

---

### 6.2 Operation log (sync backbone)

#### `aideon_ops`

Append-only.

- `partition_id` (PK part)
- `op_id` (PK part)
- `actor_id` (FK-like; not enforced cross-db)
- `asserted_at` (i64)
- `tx_id` (nullable)
- `op_type` (small int)
- `payload` (blob/text; store as bytes for portability)
- `schema_version_hint` (nullable text; optional)
- `ingested_asserted_at` (i64; when stored locally; optional)

Indexes:

- `aideon_ops__idx_partition_asserted_at` on `(partition_id, asserted_at)`
- `aideon_ops__idx_partition_actor_asserted_at` on `(partition_id, actor_id, asserted_at)`
- `aideon_ops__idx_partition_tx` on `(partition_id, tx_id)` if tx used

#### `aideon_op_deps` (optional, recommended)

- `partition_id` (PK part)
- `op_id` (PK part)
- `dep_op_id` (PK part)

Indexes:

- `aideon_op_deps__idx_partition_op` on `(partition_id, op_id)`
- `aideon_op_deps__idx_partition_dep` on `(partition_id, dep_op_id)`

---

### 6.3 Entities (nodes and edges)

#### `aideon_entities`

- `partition_id` (PK part)
- `entity_id` (PK part)
- `kind` (tiny int: 1=node, 2=edge)
- `type_id` (nullable)
- `is_deleted` (bool)
- `created_op_id` (nullable)
- `updated_op_id` (nullable)
- `created_asserted_at` (i64)
- `updated_asserted_at` (i64)

Indexes:

- `aideon_entities__idx_partition_kind_type` on `(partition_id, kind, type_id)`
- `aideon_entities__idx_partition_updated` on `(partition_id, updated_asserted_at)`
- `aideon_entities__idx_partition_type` on `(partition_id, type_id)` (useful for lists)

#### `aideon_edges`

- `partition_id` (PK part)
- `edge_id` (PK part) — must reference an entity with kind=edge (checked in code)
- `src_entity_id` (not null)
- `dst_entity_id` (not null)
- `edge_type_id` (nullable; mirrors `aideon_entities.type_id` for convenience)

Indexes (core traversal):

- `aideon_edges__idx_out` on `(partition_id, src_entity_id, edge_type_id, edge_id)`
- `aideon_edges__idx_in` on `(partition_id, dst_entity_id, edge_type_id, edge_id)`
- `aideon_edges__idx_type` on `(partition_id, edge_type_id, edge_id)`

**Endpoint immutability**

- Recommended rule: edge endpoints are immutable post-create.
- If endpoint change is required, create a new edge and tombstone the old one (preserves analytics stability and avoids cascade updates).
- Mneme enforces this by API design; no “update endpoints” method in v1.

---

### 6.4 Edge existence facts (time-valid edges)

#### `aideon_edge_exists_facts`

- `partition_id` (PK part)
- `edge_id` (PK part)
- `valid_from` (PK part, i64)
- `asserted_at` (PK part, i64)
- `op_id` (PK part)
- `valid_to` (nullable i64)
- `layer` (tiny int; higher wins)
- `is_tombstone` (bool)

Indexes:

- `aideon_edge_exists_facts__idx_edge_valid_from` on `(partition_id, edge_id, valid_from)`
- `aideon_edge_exists_facts__idx_edge_valid_to` on `(partition_id, edge_id, valid_to)`
- `aideon_edge_exists_facts__idx_partition_valid_from` on `(partition_id, valid_from)`
- Optional bucketing column (recommended for time-slider performance):
  - `valid_bucket` (i32) with index `(partition_id, valid_bucket)`
  - Mneme computes bucket in code (no computed columns), stores it.

---

### 6.5 Property facts (typed, time-valid)

Mneme stores authoritative property assignments as typed fact tables. Each table uses the same key structure.

Common columns:

- `partition_id`
- `entity_id`
- `field_id`
- `valid_from`
- `asserted_at`
- `op_id`
- `valid_to`
- `layer`
- `is_tombstone`

PK: `(partition_id, entity_id, field_id, valid_from, asserted_at, op_id)`

#### `aideon_prop_fact_str`

- `value_text` (text)

Indexes:

- `aideon_prop_fact_str__idx_entity_field_from` on `(partition_id, entity_id, field_id, valid_from)`
- `aideon_prop_fact_str__idx_entity_field_to` on `(partition_id, entity_id, field_id, valid_to)`
- `aideon_prop_fact_str__idx_field_from` on `(partition_id, field_id, valid_from)`

#### `aideon_prop_fact_i64`

- `value_i64` (bigint)

Same index set.

#### `aideon_prop_fact_f64`

- `value_f64` (double)

Same index set.

#### `aideon_prop_fact_bool`

- `value_bool` (bool)

Same index set.

#### `aideon_prop_fact_time`

- `value_time` (i64 epoch micros)

Same index set.

#### `aideon_prop_fact_ref`

- `value_ref` (Id; stored as UUID/binary/text per backend)

Same index set.

#### `aideon_prop_fact_blob`

- `value_blob` (blob)

Same index set (blob not index-filterable).

#### `aideon_prop_fact_json` (optional, discouraged for indexed fields)

- `value_json` (text)

---

### 6.6 Field indexes (only for schema-marked indexed fields)

Because fields are user-defined, indexing is opt-in via schema (`is_indexed=true`). Mneme maintains separate typed index tables for filter/sort operations.

Common columns:

- `partition_id`
- `field_id`
- `value_norm` (type-specific)
- `entity_id`
- `valid_from`
- `valid_to`
- `asserted_at`
- `layer`
- `op_id`
- `is_tombstone`

These are derived but maintained transactionally to avoid background reindex requirements. They are still logically derived from facts; rebuild is possible.

#### `aideon_idx_field_str`

- `value_norm` (text; normalised in code: case-fold, trim, optional collation key)

Indexes:

- `aideon_idx_field_str__idx_lookup` on `(partition_id, field_id, value_norm, entity_id, valid_from, asserted_at)`
- `aideon_idx_field_str__idx_field_entity` on `(partition_id, field_id, entity_id)`

Similarly:

- `aideon_idx_field_i64` (`value_norm` bigint)
- `aideon_idx_field_f64` (double)
- `aideon_idx_field_bool` (bool)
- `aideon_idx_field_time` (bigint)
- `aideon_idx_field_ref` (id)

---

### 6.7 Schema storage (metamodel as data)

#### `aideon_types`

- `partition_id` (PK part)
- `type_id` (PK part)
- `applies_to` (tiny int: 1=node, 2=edge)
- `label` (text)
- `is_abstract` (bool)
- `is_deleted` (bool)
- `updated_asserted_at` (i64)

Indexes:

- `aideon_types__idx_partition_applies` on `(partition_id, applies_to, type_id)`
- `aideon_types__idx_partition_label` on `(partition_id, label)` (exact match; normalisation handled in code if needed)

#### `aideon_type_extends`

Single inheritance.

- `partition_id` (PK part)
- `type_id` (PK part)
- `parent_type_id` (not null)

Indexes:

- `aideon_type_extends__idx_parent` on `(partition_id, parent_type_id)`

Cycle prevention is performed in code (SeaORM portable constraints cannot express recursive cycle checks).

#### `aideon_fields`

- `partition_id` (PK part)
- `field_id` (PK part)
- `label` (text)
- `value_type` (tiny int enum)
- `cardinality` (tiny int: 1=single, 2=multi)
- `merge_policy` (tiny int enum)
- `is_indexed` (bool)
- `is_deleted` (bool)
- `updated_asserted_at` (i64)

Indexes:

- `aideon_fields__idx_partition_value_type` on `(partition_id, value_type)`
- `aideon_fields__idx_partition_label` on `(partition_id, label)`

#### `aideon_type_fields`

Association and defaults/constraints.

- `partition_id` (PK part)
- `type_id` (PK part)
- `field_id` (PK part)
- `is_required` (bool)
- `default_value_kind` (tiny int: matches value_type; optional)
- `default_str` (nullable)
- `default_i64` (nullable)
- `default_f64` (nullable)
- `default_bool` (nullable)
- `default_time` (nullable)
- `default_ref` (nullable)
- `default_blob` (nullable)
- `default_json` (nullable)
- `override_default` (bool)
- `tighten_required` (bool)
- `updated_asserted_at` (i64)

Indexes:

- `aideon_type_fields__idx_type` on `(partition_id, type_id)`
- `aideon_type_fields__idx_field` on `(partition_id, field_id)`

#### `aideon_edge_type_rules` (recommended for analytics integrity)

Defines semantic direction labels and optional endpoint constraints.

- `partition_id` (PK part)
- `edge_type_id` (PK part)
- `semantic_direction` (text) — e.g., “influences”, “realises”, “enables”
- `src_type_ids_json` (nullable text) — list of allowed src type IDs (stored as JSON array of strings)
- `dst_type_ids_json` (nullable text)
- `updated_asserted_at` (i64)

Note: JSON is stored as text across all DBs; enforcement is in code.

---

### 6.8 Effective schema cache (derived)

#### `aideon_effective_schema_cache`

Derived flattened schema per type and schema-version hash.

- `partition_id` (PK part)
- `type_id` (PK part)
- `schema_version_hash` (PK part; text)
- `blob` (blob/text; recommend msgpack bytes)
- `built_asserted_at` (i64)

---

### 6.9 Analytics projection tables (authoritative-ish derived)

#### `aideon_graph_projection_edges`

Directed adjacency list for analytics.

- `partition_id` (PK part)
- `edge_id` (PK part) — stable identity for weighting and filtering
- `src_entity_id` (not null)
- `dst_entity_id` (not null)
- `edge_type_id` (nullable)
- `weight` (double; default 1.0)
- `updated_asserted_at` (i64)

Indexes:

- `aideon_graph_projection_edges__idx_src` on `(partition_id, src_entity_id)`
- `aideon_graph_projection_edges__idx_dst` on `(partition_id, dst_entity_id)`
- `aideon_graph_projection_edges__idx_type_src` on `(partition_id, edge_type_id, src_entity_id)`

Population:

- Insert/update in the same transaction when creating/tombstoning an edge entity.
- If endpoint immutability holds, projection rows are stable.

#### `aideon_pagerank_runs` (derived, optional)

- `partition_id` (PK part)
- `run_id` (PK part)
- `as_of_valid_time` (nullable i64)
- `as_of_asserted_at` (nullable i64)
- `params_json` (text)
- `created_asserted_at` (i64)

#### `aideon_pagerank_scores`

- `partition_id` (PK part)
- `run_id` (PK part)
- `entity_id` (PK part)
- `score` (double)

Indexes:

- `aideon_pagerank_scores__idx_run_score` on `(partition_id, run_id, score)` (descending not portable; sort in query)

---

## 7. Migrations (SeaORM + SeaQuery, no raw SQL)

### 7.1 Migration toolchain

- Use `sea_orm_migration` with `MigratorTrait`.
- Each migration:
  - creates/alter tables using `Table::create`, `Table::alter`, `Index::create`, `ForeignKey::create` where cross-db compatible.

- Avoid unsupported constraints:
  - CHECK constraints are inconsistently supported; prefer code validation.
  - Partial indexes are not portable; avoid.

### 7.2 Schema versioning

Mneme uses SeaORM’s migration tracking plus an explicit table for diagnostics.

#### `aideon_schema_version`

- `version` (PK, text)
- `applied_asserted_at` (i64)
- `checksum` (text)
- `app_version` (text, optional)

### 7.3 Cross-platform type strategy

- IDs: store as SeaORM `Uuid` for Postgres; for SQLite/MySQL use `String` or `Vec<u8>` consistently through feature-gated entity definitions and conversion helpers.
- All time columns: `BigInteger`.
- Payload/blob: `Binary` where supported; for SQLite fallback to `Blob`.

### 7.4 Index strategy (portable)

- Use composite B-tree indexes only.
- Avoid functional indexes; perform normalisation in code and store `value_norm`.

---

## 8. Runtime architecture and responsibilities

### 8.1 Write path (transactional)

Every Mneme write API call:

1. opens a DB transaction via SeaORM
2. inserts an operation row in `aideon_ops`
3. applies corresponding inserts into state tables (entities, facts, indexes, projection rows)
4. commits or rolls back atomically

No background worker is required for correctness.

### 8.2 Read path

Reads never expose SQL:

- Mneme resolves values at valid-time and optional asserted-time via SeaORM queries.
- Defaults are resolved using effective schema cache (derived).

### 8.3 Strictness modes (validation)

Mneme supports validation levels (configurable per partition):

- `OFF`: store without schema validation (still enforces basic integrity)
- `WARN`: store but emit validation warnings
- `ERROR`: reject writes that violate schema rules

Schema rules supported in v1:

- abstract types cannot be instantiated
- field value type matches
- cardinality enforced for single vs multi (single cannot have two winners at same valid time unless MV policy explicitly)
- (optional) edge endpoint allowed types via `aideon_edge_type_rules` constraints

---

## 9. API: Mneme surface to the rest of the system

No consumer modules generate SQL. All access is via APIs below.

### 9.1 Rust crate layout (suggested)

- `mneme_core` (types, time, IDs, encoding)
- `mneme_store` (SeaORM models, repositories, migrations)
- `mneme_api` (public Rust API traits + structs)
- `mneme_ts` (bindings: wasm-bindgen or napi-rs)
- `mneme_sync` (op export/ingest)
- `mneme_analytics` (projection access, score storage)

### 9.2 Rust public types

```rust
pub struct PartitionId(pub String);
pub struct ActorId(pub String);
pub struct OpId(pub String);
pub struct EntityId(pub String);
pub struct TypeId(pub String);
pub struct FieldId(pub String);

pub struct ValidTime(pub i64);    // epoch micros UTC
pub struct AssertedTime(pub i64); // HLC encoded as i64

#[derive(Clone, Copy)]
pub enum Layer { Plan = 10, Actual = 20 }

#[derive(Clone, Copy)]
pub enum EntityKind { Node = 1, Edge = 2 }

#[derive(Clone, Copy)]
pub enum ValueType { Str=1, I64=2, F64=3, Bool=4, Time=5, Ref=6, Blob=7, Json=8 }

#[derive(Clone, Copy)]
pub enum MergePolicy { Lww=1, Mv=2, OrSet=3, Counter=4, Text=5 }

#[derive(Clone)]
pub enum Value {
  Str(String),
  I64(i64),
  F64(f64),
  Bool(bool),
  Time(i64),
  Ref(String),
  Blob(Vec<u8>),
  Json(serde_json::Value),
}

pub enum ReadValue {
  Single(Value),
  Multi(Vec<Value>),
}
```

---

## 10. API: Metamodel ingestion (Praxis → Mneme)

### 10.1 Upsert metamodel batch

Praxis submits a batch that Mneme upserts transactionally.

```rust
pub struct TypeDef {
  pub type_id: TypeId,
  pub applies_to: EntityKind,
  pub label: String,
  pub is_abstract: bool,
  pub parent_type_id: Option<TypeId>,
}

pub struct FieldDef {
  pub field_id: FieldId,
  pub label: String,
  pub value_type: ValueType,
  pub cardinality_multi: bool,
  pub merge_policy: MergePolicy,
  pub is_indexed: bool,
}

pub struct TypeFieldDef {
  pub type_id: TypeId,
  pub field_id: FieldId,
  pub is_required: bool,
  pub default_value: Option<Value>,
  pub override_default: bool,
  pub tighten_required: bool,
}

pub struct EdgeTypeRuleDef {
  pub edge_type_id: TypeId,
  pub semantic_direction: String,
  pub allowed_src_type_ids: Vec<TypeId>, // empty = any
  pub allowed_dst_type_ids: Vec<TypeId>, // empty = any
}

pub struct MetamodelBatch {
  pub types: Vec<TypeDef>,
  pub fields: Vec<FieldDef>,
  pub type_fields: Vec<TypeFieldDef>,
  pub edge_type_rules: Vec<EdgeTypeRuleDef>,
}

pub struct SchemaCompileResult {
  pub type_id: TypeId,
  pub schema_version_hash: String,
}

pub trait MnemeMetamodelApi {
  fn upsert_metamodel_batch(
    &self,
    partition: PartitionId,
    actor: ActorId,
    asserted_at: AssertedTime,
    batch: MetamodelBatch,
  ) -> Result<OpId, MnemeError>;

  fn compile_effective_schema(
    &self,
    partition: PartitionId,
    actor: ActorId,
    asserted_at: AssertedTime,
    type_id: TypeId,
  ) -> Result<SchemaCompileResult, MnemeError>;
}
```

### 10.2 Effective schema semantics

- Single inheritance chain is flattened.
- Default values are resolved with child overrides.
- Constraint tightening allowed; widening rejected in `ERROR` mode.

---

## 11. API: Graph writes (entities, edges, existence)

### 11.1 Create node

```rust
pub struct CreateNodeInput {
  pub partition: PartitionId,
  pub actor: ActorId,
  pub asserted_at: AssertedTime,
  pub node_id: EntityId,
  pub type_id: Option<TypeId>,
}

pub trait MnemeGraphWriteApi {
  fn create_node(&self, input: CreateNodeInput) -> Result<OpId, MnemeError>;
}
```

### 11.2 Create edge (with existence interval)

```rust
pub struct CreateEdgeInput {
  pub partition: PartitionId,
  pub actor: ActorId,
  pub asserted_at: AssertedTime,
  pub edge_id: EntityId,
  pub type_id: Option<TypeId>,
  pub src_id: EntityId,
  pub dst_id: EntityId,
  pub exists_valid_from: ValidTime,
  pub exists_valid_to: Option<ValidTime>,
  pub layer: Layer,
  pub weight: Option<f64>, // for analytics projection (default 1.0)
}

pub trait MnemeGraphWriteApi {
  fn create_edge(&self, input: CreateEdgeInput) -> Result<OpId, MnemeError>;
}
```

### 11.3 Tombstone entity

```rust
pub struct TombstoneEntityInput {
  pub partition: PartitionId,
  pub actor: ActorId,
  pub asserted_at: AssertedTime,
  pub entity_id: EntityId,
}

pub trait MnemeGraphWriteApi {
  fn tombstone_entity(&self, input: TombstoneEntityInput) -> Result<OpId, MnemeError>;
}
```

### 11.4 Modify edge existence (plateaus)

Instead of changing endpoints, edge existence is altered by inserting facts.

```rust
pub struct SetEdgeExistenceIntervalInput {
  pub partition: PartitionId,
  pub actor: ActorId,
  pub asserted_at: AssertedTime,
  pub edge_id: EntityId,
  pub valid_from: ValidTime,
  pub valid_to: Option<ValidTime>,
  pub layer: Layer,
  pub is_tombstone: bool,
}

pub trait MnemeGraphWriteApi {
  fn set_edge_existence_interval(&self, input: SetEdgeExistenceIntervalInput) -> Result<OpId, MnemeError>;
}
```

---

## 12. API: Property writes (time-valid facts)

### 12.1 Set property interval

```rust
pub struct SetPropertyIntervalInput {
  pub partition: PartitionId,
  pub actor: ActorId,
  pub asserted_at: AssertedTime,
  pub entity_id: EntityId,
  pub field_id: FieldId,
  pub value: Value,
  pub valid_from: ValidTime,
  pub valid_to: Option<ValidTime>,
  pub layer: Layer,
}

pub struct ClearPropertyIntervalInput {
  pub partition: PartitionId,
  pub actor: ActorId,
  pub asserted_at: AssertedTime,
  pub entity_id: EntityId,
  pub field_id: FieldId,
  pub valid_from: ValidTime,
  pub valid_to: Option<ValidTime>,
  pub layer: Layer,
}

pub trait MnemePropertyWriteApi {
  fn set_property_interval(&self, input: SetPropertyIntervalInput) -> Result<OpId, MnemeError>;
  fn clear_property_interval(&self, input: ClearPropertyIntervalInput) -> Result<OpId, MnemeError>;
}
```

### 12.2 OR-Set and counter support (storage-level)

If a field’s merge policy is `OR_SET` or `COUNTER`, Mneme supports dedicated writes.

#### OR-Set element facts

Mneme stores set membership changes as facts in typed set-element tables (v1 optional; recommended if sets are needed).

```rust
pub enum SetElementOp { Add=1, Remove=2 }

pub struct UpdateOrSetElementInput {
  pub partition: PartitionId,
  pub actor: ActorId,
  pub asserted_at: AssertedTime,
  pub entity_id: EntityId,
  pub field_id: FieldId,
  pub element: Value,
  pub op: SetElementOp,
  pub valid_from: ValidTime,
  pub valid_to: Option<ValidTime>,
  pub layer: Layer,
}

pub trait MnemePropertyWriteApi {
  fn update_or_set_element(&self, input: UpdateOrSetElementInput) -> Result<OpId, MnemeError>;
}
```

#### Counter delta facts

```rust
pub struct AddCounterDeltaInput {
  pub partition: PartitionId,
  pub actor: ActorId,
  pub asserted_at: AssertedTime,
  pub entity_id: EntityId,
  pub field_id: FieldId,
  pub delta: i64,
  pub valid_from: ValidTime,
  pub valid_to: Option<ValidTime>,
  pub layer: Layer,
}

pub trait MnemePropertyWriteApi {
  fn add_counter_delta(&self, input: AddCounterDeltaInput) -> Result<OpId, MnemeError>;
}
```

---

## 13. API: Reads and traversal (time travel)

### 13.1 Read entity at valid time

```rust
pub struct ReadEntityAtTimeInput {
  pub partition: PartitionId,
  pub entity_id: EntityId,
  pub at_valid_time: ValidTime,
  pub as_of_asserted_at: Option<AssertedTime>,
  pub field_ids: Option<Vec<FieldId>>,
  pub include_defaults: bool,
}

pub struct ReadEntityAtTimeResult {
  pub entity_id: EntityId,
  pub kind: EntityKind,
  pub type_id: Option<TypeId>,
  pub is_deleted: bool,
  pub properties: std::collections::HashMap<FieldId, ReadValue>,
}

pub trait MnemeReadApi {
  fn read_entity_at_time(&self, input: ReadEntityAtTimeInput) -> Result<ReadEntityAtTimeResult, MnemeError>;
}
```

### 13.2 Traverse edges at valid time

Traversal filters edges by:

- adjacency from `aideon_edges`
- time validity from `aideon_edge_exists_facts`
- optional edge type filter

```rust
pub enum Direction { Out=1, In=2 }

pub struct TraverseAtTimeInput {
  pub partition: PartitionId,
  pub from_entity_id: EntityId,
  pub direction: Direction,
  pub edge_type_id: Option<TypeId>,
  pub at_valid_time: ValidTime,
  pub as_of_asserted_at: Option<AssertedTime>,
  pub limit: u32,
}

pub struct TraverseEdgeItem {
  pub edge_id: EntityId,
  pub src_id: EntityId,
  pub dst_id: EntityId,
  pub edge_type_id: Option<TypeId>,
}

pub trait MnemeReadApi {
  fn traverse_at_time(&self, input: TraverseAtTimeInput) -> Result<Vec<TraverseEdgeItem>, MnemeError>;
}
```

### 13.3 Field-indexed queries (filtering lists)

Mneme provides list queries over indexed fields without exposing SQL.

```rust
pub enum CompareOp { Eq, Ne, Lt, Lte, Gt, Gte, Prefix, Contains }

pub struct FieldFilter {
  pub field_id: FieldId,
  pub op: CompareOp,
  pub value: Value,
}

pub struct ListEntitiesInput {
  pub partition: PartitionId,
  pub kind: Option<EntityKind>,
  pub type_id: Option<TypeId>,
  pub at_valid_time: ValidTime,
  pub as_of_asserted_at: Option<AssertedTime>,
  pub filters: Vec<FieldFilter>,  // must refer to indexed fields for performance; otherwise rejected or warned
  pub limit: u32,
  pub cursor: Option<String>,
}

pub struct ListEntitiesResultItem {
  pub entity_id: EntityId,
  pub kind: EntityKind,
  pub type_id: Option<TypeId>,
}

pub trait MnemeReadApi {
  fn list_entities(&self, input: ListEntitiesInput) -> Result<Vec<ListEntitiesResultItem>, MnemeError>;
}
```

---

## 14. API: Analytics projections (PageRank-ready adjacency)

### 14.1 Get projection edges

Mneme returns directed adjacency suitable for PageRank and other graph algorithms. Consumers do not query tables directly.

```rust
pub struct ProjectionEdge {
  pub edge_id: EntityId,
  pub src_id: EntityId,
  pub dst_id: EntityId,
  pub edge_type_id: Option<TypeId>,
  pub weight: f64,
}

pub struct GetProjectionEdgesInput {
  pub partition: PartitionId,
  pub at_valid_time: Option<ValidTime>,         // None => current
  pub as_of_asserted_at: Option<AssertedTime>,  // audit if set
  pub edge_type_filter: Option<Vec<TypeId>>,
  pub limit: Option<u32>,
}

pub trait MnemeAnalyticsApi {
  fn get_projection_edges(&self, input: GetProjectionEdgesInput) -> Result<Vec<ProjectionEdge>, MnemeError>;
}
```

### 14.2 Store and retrieve PageRank results (optional)

```rust
pub struct PageRankParams {
  pub damping: f64,
  pub max_iters: u32,
  pub tol: f64,
  pub personalised_seed: Option<Vec<(EntityId, f64)>>,
}

pub struct StorePageRankRunInput {
  pub partition: PartitionId,
  pub actor: ActorId,
  pub asserted_at: AssertedTime,
  pub as_of_valid_time: Option<ValidTime>,
  pub as_of_asserted_at: Option<AssertedTime>,
  pub params: PageRankParams,
  pub scores: Vec<(EntityId, f64)>,
}

pub trait MnemeAnalyticsApi {
  fn store_pagerank_run(&self, input: StorePageRankRunInput) -> Result<String /*run_id*/, MnemeError>;
  fn get_pagerank_scores(&self, partition: PartitionId, run_id: String, top_n: u32) -> Result<Vec<(EntityId, f64)>, MnemeError>;
}
```

---

## 15. API: Sync (operation log export/ingest)

Mneme uses a partition-scoped operation stream.

```rust
pub struct OpEnvelope {
  pub op_id: OpId,
  pub actor_id: ActorId,
  pub asserted_at: AssertedTime,
  pub op_type: u16,
  pub payload: Vec<u8>,
  pub deps: Vec<OpId>,
}

pub struct ExportOpsInput {
  pub partition: PartitionId,
  pub since_asserted_at: Option<AssertedTime>,
  pub limit: u32,
}

pub trait MnemeSyncApi {
  fn export_ops(&self, input: ExportOpsInput) -> Result<Vec<OpEnvelope>, MnemeError>;
  fn ingest_ops(&self, partition: PartitionId, ops: Vec<OpEnvelope>) -> Result<(), MnemeError>;
  fn get_partition_head(&self, partition: PartitionId) -> Result<AssertedTime, MnemeError>;
}
```

Ingestion rules:

- ops are idempotent by PK `(partition_id, op_id)`
- applying an op replays its state effects transactionally (or skips if already applied)
- schema ops trigger schema cache invalidation

---

## 16. Scenario / what-if modelling (no Git concepts)

Mneme supports scenarios via an explicit `scenario_id` overlay within the same partition (portable across DBs and avoids cross-partition joins).

### 16.1 Tables

Add `scenario_id` (nullable) to:

- `aideon_ops`
- all fact tables (`aideon_edge_exists_facts`, `aideon_prop_fact_*`, indexes)
- optionally `aideon_entities` for scenario-only entities (recommended: entities are scenario-scoped if created under scenario)

Key semantic:

- Baseline = `scenario_id IS NULL`
- Scenario overlay = `scenario_id = X`
- Reads in scenario mode resolve facts by:
  1. scenario facts (scenario_id=X)
  2. fallback to baseline facts (scenario_id NULL)

### 16.2 API additions

```rust
pub struct ScenarioId(pub String);

pub struct CreateScenarioInput {
  pub partition: PartitionId,
  pub actor: ActorId,
  pub asserted_at: AssertedTime,
  pub name: String,
}

pub trait MnemeScenarioApi {
  fn create_scenario(&self, input: CreateScenarioInput) -> Result<ScenarioId, MnemeError>;
  fn delete_scenario(&self, partition: PartitionId, actor: ActorId, asserted_at: AssertedTime, scenario: ScenarioId) -> Result<(), MnemeError>;
}
```

All read/write APIs accept optional `scenario_id`:

- if present, writes go to that scenario overlay
- reads use scenario-first resolution

---

## 17. Required and optional fields (explicit)

### 17.1 Required fields for node creation

- `partition_id`
- `actor_id`
- `asserted_at`
- `node_id`
  Optional:
- `type_id`

### 17.2 Required fields for edge creation

- `partition_id`
- `actor_id`
- `asserted_at`
- `edge_id`
- `src_id`
- `dst_id`
- `exists_valid_from`
  Optional:
- `exists_valid_to`
- `type_id`
- `layer` (default Actual)
- `weight` (default 1.0)

### 17.3 Required fields for property interval

- `partition_id`
- `actor_id`
- `asserted_at`
- `entity_id`
- `field_id`
- `value`
- `valid_from`
  Optional:
- `valid_to`
- `layer` (default Actual)

### 17.4 Metamodel fields

Required:

- `type_id`, `applies_to`, `label`, `is_abstract`
- `field_id`, `label`, `value_type`, `cardinality`, `merge_policy`, `is_indexed`
- `type_id + field_id` attachments with at least required flag
  Optional:
- `parent_type_id`
- `default_value`
- edge-type rules

---

## 18. Performance requirements and indexing strategy (portable)

### 18.1 Adjacency traversal

Traversal must be O(degree) for:

- out edges by `(partition, src, type)`
- in edges by `(partition, dst, type)`
  Indexes in `aideon_edges` are mandatory.

### 18.2 Time slice filtering

Edges must filter by existence facts at time T:

- query existence facts by `(partition, edge_id)` + containment
  Indexes on `(partition, edge_id, valid_from)` and `(partition, edge_id, valid_to)` are mandatory.

### 18.3 Time-slider optimisation (portable)

Mneme must implement **bucketed containment**:

- `valid_bucket` computed in code for fact rows
- typical buckets:
  - day bucket for UI scrubbing
  - optional week/month for higher levels
    The bucket column and index allow limiting candidate rows before containment filtering.

### 18.4 Field filtering

Only indexed fields are eligible for fast filter queries:

- Mneme rejects or warns (depending on strictness) if filters reference non-indexed fields.
- Index tables store `value_norm` computed in code.

### 18.5 PageRank readiness

Analytics adjacency retrieval must not require joining large property tables:

- use `aideon_graph_projection_edges` + existence filtering
- optionally filter by edge types relevant to EA metamodel rules

---

## 19. Data integrity and validation (portable constraints + code)

Because portable SQL constraints are limited, Mneme enforces most rules in code:

- `aideon_entities.kind` must match presence in `aideon_edges`
- abstract types cannot be instantiated
- property value type must match field definition
- inheritance must be acyclic and single-parent
- edge endpoints may be validated against `aideon_edge_type_rules` if strictness requires

Foreign keys:

- Use where SeaORM can generate them portably without breaking SQLite pragmas.
- Mneme should not rely on FK enforcement for correctness; it is an optimisation.

---

## 20. TypeScript API spec (`@aideon/mneme`)

Bindings are generated via WASM (wasm-bindgen) or native addon (napi-rs). The API mirrors Rust and hides DB details.

### 20.1 TS types

```ts
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

export type ReadValue = { k: 'single'; v: Value } | { k: 'multi'; v: Value[] };
```

### 20.2 Metamodel API

```ts
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

export interface MnemeMetamodelApi {
  upsertMetamodelBatch(input: {
    partitionId: PartitionId;
    actorId: ActorId;
    assertedAt: AssertedTime;
    batch: MetamodelBatch;
    scenarioId?: ScenarioId;
  }): Promise<{ opId: OpId }>;

  compileEffectiveSchema(input: {
    partitionId: PartitionId;
    actorId: ActorId;
    assertedAt: AssertedTime;
    typeId: TypeId;
    scenarioId?: ScenarioId;
  }): Promise<{ schemaVersionHash: string }>;
}
```

### 20.3 Write API

```ts
export interface MnemeWriteApi {
  createNode(input: {
    partitionId: PartitionId;
    actorId: ActorId;
    assertedAt: AssertedTime;
    nodeId: EntityId;
    typeId?: TypeId;
    scenarioId?: ScenarioId;
  }): Promise<{ opId: OpId }>;

  createEdge(input: {
    partitionId: PartitionId;
    actorId: ActorId;
    assertedAt: AssertedTime;
    edgeId: EntityId;
    typeId?: TypeId;
    srcId: EntityId;
    dstId: EntityId;
    existsValidFrom: ValidTime;
    existsValidTo?: ValidTime;
    layer?: Layer;
    weight?: number;
    scenarioId?: ScenarioId;
  }): Promise<{ opId: OpId }>;

  setEdgeExistenceInterval(input: {
    partitionId: PartitionId;
    actorId: ActorId;
    assertedAt: AssertedTime;
    edgeId: EntityId;
    validFrom: ValidTime;
    validTo?: ValidTime;
    layer?: Layer;
    isTombstone?: boolean;
    scenarioId?: ScenarioId;
  }): Promise<{ opId: OpId }>;

  tombstoneEntity(input: {
    partitionId: PartitionId;
    actorId: ActorId;
    assertedAt: AssertedTime;
    entityId: EntityId;
    scenarioId?: ScenarioId;
  }): Promise<{ opId: OpId }>;

  setPropertyInterval(input: {
    partitionId: PartitionId;
    actorId: ActorId;
    assertedAt: AssertedTime;
    entityId: EntityId;
    fieldId: FieldId;
    value: Value;
    validFrom: ValidTime;
    validTo?: ValidTime;
    layer?: Layer;
    scenarioId?: ScenarioId;
  }): Promise<{ opId: OpId }>;

  clearPropertyInterval(input: {
    partitionId: PartitionId;
    actorId: ActorId;
    assertedAt: AssertedTime;
    entityId: EntityId;
    fieldId: FieldId;
    validFrom: ValidTime;
    validTo?: ValidTime;
    layer?: Layer;
    scenarioId?: ScenarioId;
  }): Promise<{ opId: OpId }>;
}
```

### 20.4 Read API

```ts
export interface MnemeReadApi {
  readEntityAtTime(input: {
    partitionId: PartitionId;
    entityId: EntityId;
    at: ValidTime;
    asOfAssertedAt?: AssertedTime;
    fieldIds?: FieldId[];
    includeDefaults?: boolean;
    scenarioId?: ScenarioId;
  }): Promise<{
    entityId: EntityId;
    kind: 'Node' | 'Edge';
    typeId?: TypeId;
    isDeleted: boolean;
    properties: Record<FieldId, ReadValue>;
  }>;

  traverseAtTime(input: {
    partitionId: PartitionId;
    fromEntityId: EntityId;
    direction: 'out' | 'in';
    edgeTypeId?: TypeId;
    at: ValidTime;
    asOfAssertedAt?: AssertedTime;
    limit?: number;
    scenarioId?: ScenarioId;
  }): Promise<Array<{ edgeId: EntityId; srcId: EntityId; dstId: EntityId; edgeTypeId?: TypeId }>>;

  listEntities(input: {
    partitionId: PartitionId;
    at: ValidTime;
    asOfAssertedAt?: AssertedTime;
    kind?: 'Node' | 'Edge';
    typeId?: TypeId;
    filters?: Array<{
      fieldId: FieldId;
      op: 'Eq' | 'Ne' | 'Lt' | 'Lte' | 'Gt' | 'Gte' | 'Prefix' | 'Contains';
      value: Value;
    }>;
    limit?: number;
    cursor?: string;
    scenarioId?: ScenarioId;
  }): Promise<Array<{ entityId: EntityId; kind: 'Node' | 'Edge'; typeId?: TypeId }>>;
}
```

### 20.5 Analytics API

```ts
export interface MnemeAnalyticsApi {
  getProjectionEdges(input: {
    partitionId: PartitionId;
    at?: ValidTime;
    asOfAssertedAt?: AssertedTime;
    edgeTypeFilter?: TypeId[];
    limit?: number;
    scenarioId?: ScenarioId;
  }): Promise<
    Array<{
      edgeId: EntityId;
      srcId: EntityId;
      dstId: EntityId;
      edgeTypeId?: TypeId;
      weight: number;
    }>
  >;

  storePageRankRun?(input: {
    partitionId: PartitionId;
    actorId: ActorId;
    assertedAt: AssertedTime;
    asOfValidTime?: ValidTime;
    asOfAssertedAt?: AssertedTime;
    params: {
      damping: number;
      maxIters: number;
      tol: number;
      personalisedSeed?: Array<{ id: EntityId; w: number }>;
    };
    scores: Array<{ id: EntityId; score: number }>;
    scenarioId?: ScenarioId;
  }): Promise<{ runId: Id }>;

  getPageRankScores?(input: {
    partitionId: PartitionId;
    runId: Id;
    topN: number;
    scenarioId?: ScenarioId;
  }): Promise<Array<{ id: EntityId; score: number }>>;
}
```

### 20.6 Sync API

```ts
export interface MnemeSyncApi {
  exportOps(input: {
    partitionId: PartitionId;
    sinceAssertedAt?: AssertedTime;
    limit?: number;
    scenarioId?: ScenarioId;
  }): Promise<{
    ops: Array<{
      opId: OpId;
      actorId: ActorId;
      assertedAt: AssertedTime;
      opType: number;
      payload: Uint8Array;
      deps: OpId[];
    }>;
  }>;

  ingestOps(input: {
    partitionId: PartitionId;
    ops: Array<{
      opId: OpId;
      actorId: ActorId;
      assertedAt: AssertedTime;
      opType: number;
      payload: Uint8Array;
      deps: OpId[];
    }>;
    scenarioId?: ScenarioId;
  }): Promise<void>;

  getPartitionHead(input: {
    partitionId: PartitionId;
    scenarioId?: ScenarioId;
  }): Promise<{ head: AssertedTime }>;
}
```

---

## 21. Test specifications (storage-engine focused)

### 21.1 Migration portability tests

Run migration suite against:

- SQLite
- Postgres
- MySQL/MariaDB

Verify:

- all `aideon_*` tables exist
- all required indexes exist (name-based or introspection-based via SeaORM schema discovery where possible)
- migrations are repeatable (no-op when already applied)

### 21.2 Transactional integrity tests

For each write API:

- inject a failure after op insert but before fact insert
- assert rollback: no partial rows
- inject failure after fact insert but before projection/index insert
- assert rollback: no partial rows

### 21.3 Bi-temporal correctness tests

- Backdated correction (asserted later, valid earlier):
  - playback vs audit results differ correctly

- Overlapping plateaus:
  - layer precedence works
  - specificity works
  - deterministic tie-break works

### 21.4 Traversal at time tests

- edges exist only within certain valid intervals
- traversal returns only edges active at `T`
- audit mode filters by asserted time

### 21.5 Field index correctness tests

- indexed field writes populate `aideon_idx_field_*`
- filtering uses index tables only
- non-indexed field filters rejected/warned as configured

### 21.6 Projection correctness tests (analytics)

- creating edge creates projection row
- tombstoning edge removes or marks projection unusable (Mneme chooses one consistent behaviour; recommended: keep projection row but rely on edge existence facts + entity tombstone for activeness)
- adjacency retrieval at time T respects existence facts

### 21.7 Idempotent ingestion tests

- ingest same op twice
- state unchanged after second ingest; no duplicate PK violations leak as errors (skip on conflict)

---

## 22. Implementation notes (SeaORM-first, no raw SQL)

- Use `DatabaseTransaction` everywhere for writes.
- Prefer `insert_many` / batched inserts for performance.
- For “insert-if-not-exists” on ops ingestion:
  - use SeaORM’s `OnConflict` support (portable).

- Avoid DB-specific features:
  - no partial indexes
  - no generated columns
  - no recursive CTE reliance in public API (can be added later if SeaORM supports it portably)

- Normalise strings for indexed fields in code and store `value_norm`.
- Use explicit pagination cursors (opaque) rather than relying on DB-specific offset performance.

---

## 23. Summary of Mneme’s guarantees (contract)

- No other module emits SQL; all access is via Mneme APIs.
- All writes are ACID and produce an operation log record.
- Reads can resolve state at any valid time, with optional audit “as-of asserted time”.
- Graph traversal is efficient and time-aware.
- Analytics adjacency is available as a stable directed projection suitable for PageRank and other dependency-based measures.
- The storage engine is portable across SQLite, Postgres, and MySQL/MariaDB using only SeaORM/SeaQuery features.

---

## 24. Triggered processing and derived artefact pipelines

### 24.1 Purpose

Mneme must support **triggered processing** to keep derived artefacts up to date without requiring other modules to query raw facts or run SQL. This includes:

- flattening/compiling metamodel (effective schema caches)
- maintaining analytics-friendly projections (directed adjacency, degree stats, connectivity checks)
- maintaining derived indexes beyond basic field indexes (where justified)
- preparing incremental “deltas” for downstream analytics modules
- supporting near-real-time processing with predictable latency

Triggered processing is implemented **in Mneme application code**, not via database triggers, to remain portable across SQLite/Postgres/MySQL and avoid raw SQL.

### 24.2 Design principles

1. **Portable**: no DB triggers, no vendor-specific scheduling, no raw SQL.
2. **Deterministic**: derived artefacts must be reproducible from authoritative data.
3. **Transactional correctness**:
   - Writes commit authoritative data first.
   - Derived updates may be:
     - synchronous within the same transaction (strong consistency)
     - asynchronous but guaranteed (eventual derived consistency)

4. **Explicit contracts**:
   - each derived artefact declares:
     - inputs (tables/events)
     - outputs (tables)
     - consistency mode (sync vs async)
     - recompute strategy (incremental vs full rebuild)

5. **Backpressure**:
   - analytics and compilation jobs must not block core writes indefinitely.
   - queue and worker model must allow bounded work per commit.

### 24.3 Derived artefact categories

Mneme distinguishes three categories with different consistency expectations:

#### A) Must-be-consistent (sync-in-tx)

Updated inside the same DB transaction as the write:

- field index tables (`aideon_idx_field_*`) for `is_indexed=true` fields
- graph projection edges (`aideon_graph_projection_edges`) when edges created/tombstoned
- basic entity metadata updates (`aideon_entities.updated_*`)
- per-partition “heads”/progress markers if stored

Rationale: these directly support user-facing queries and should not lag.

#### B) Near-real-time (async but fast)

Updated shortly after commit by a local worker, within bounded time:

- effective schema cache rebuilds for impacted types
- derived connectivity checks (end-to-end paths completeness signals)
- materialised “current” views for common queries (optional)
- incremental analytics pre-aggregations (e.g., degree counts, type counts)

Rationale: small delays acceptable; keeps write latency low.

#### C) Batch/offline (async, scheduled or on-demand)

Rebuilt periodically or on request:

- PageRank scores and other global graph algorithms
- expensive integrity audits across whole partitions
- compacted snapshots for time slider performance (if implemented)

Rationale: these are heavy and need scheduling and resource control.

### 24.4 Trigger sources

Triggered processing is initiated from **committed operations**. The primary trigger signal is the row(s) written to `aideon_ops`.

Mneme defines a canonical trigger message:

```rust
pub struct TriggerEvent {
  pub partition_id: PartitionId,
  pub scenario_id: Option<ScenarioId>,
  pub op_id: OpId,
  pub op_type: u16,
  pub asserted_at: AssertedTime,
  pub entity_id: Option<EntityId>,
  pub type_id: Option<TypeId>,
  pub field_id: Option<FieldId>,
}
```

Events are created from the write pipeline (or ingest pipeline) after successful commit.

### 24.5 Internal job queue (portable)

Mneme implements a **database-backed job queue** using SeaORM tables, so it works in all supported DBs and survives restarts.

#### 24.5.1 Tables

##### `aideon_jobs`

- `partition_id` (PK part)
- `job_id` (PK part)
- `job_type` (text or small int)
- `status` (tiny int: pending/running/succeeded/failed)
- `priority` (int)
- `attempts` (int)
- `max_attempts` (int)
- `lease_expires_at` (i64, nullable) — for worker leasing
- `created_asserted_at` (i64)
- `updated_asserted_at` (i64)
- `payload` (blob/text) — job parameters
  Indexes:
- `aideon_jobs__idx_pending` on `(partition_id, status, priority, created_asserted_at)`
- `aideon_jobs__idx_lease` on `(partition_id, lease_expires_at)`

##### `aideon_job_events` (optional, for observability)

- `partition_id`
- `job_id`
- `event_time` (i64)
- `message` (text)

#### 24.5.2 Leasing and concurrency

Workers claim jobs by:

- selecting pending jobs ordered by priority
- setting `status=running` and `lease_expires_at = now + lease_duration` in a transaction
- renewing leases if long running
- marking succeeded/failed with backoff and retry

All of this is implemented with SeaORM updates and `OnConflict` patterns; no raw SQL.

### 24.6 Triggered processing registry

Mneme defines a registry of processors:

```rust
pub trait Processor {
  fn name(&self) -> &'static str;
  fn interested_in(&self, op_type: u16) -> bool;
  fn plan_jobs(&self, evt: &TriggerEvent) -> Vec<JobSpec>; // lightweight
  fn run_job(&self, ctx: &JobContext, job: &JobRecord) -> Result<(), MnemeError>;
}

pub struct JobSpec {
  pub job_type: String,
  pub priority: i32,
  pub payload: Vec<u8>,
  pub dedupe_key: Option<String>,
}
```

Key requirement: `plan_jobs` must be fast and must not execute heavy DB scans.

### 24.7 Dedupe and coalescing

To avoid job storms during bulk ingest or batch metamodel updates:

- processors emit a `dedupe_key` (e.g., `schema:partition:type_id`)
- Mneme enforces “at most one pending/running job per dedupe key” by storing the key in the job row and using SeaORM `OnConflict` to upsert.

Add column:

- `dedupe_key` (nullable text)
  Index:
- `(partition_id, job_type, dedupe_key, status)` to support conflict checks (portable)

### 24.8 Processor specifications (required in v1)

#### 24.8.1 Effective schema compiler processor

**Purpose:** flatten/compile type inheritance + field attachments into cached effective schema.

**Triggers:**

- metamodel upsert ops affecting:
  - `aideon_types`
  - `aideon_type_extends`
  - `aideon_fields`
  - `aideon_type_fields`
  - `aideon_edge_type_rules` (if schema compilation includes edge semantics)

**Outputs:**

- `aideon_effective_schema_cache`

**Consistency mode:**

- near-real-time async (Category B), with optional synchronous compile for callers that require immediate availability.

**Job payload:**

- `(partition_id, scenario_id, type_id | ALL, schema_version_hint)`

**Algorithm:**

1. Load type graph for impacted type(s) (single inheritance chain).
2. Collect fields base→derived, apply overrides.
3. Produce `schema_version_hash`:
   - hash of relevant rows (type + ancestors + attached fields + field defs)
   - implemented in code using stable serialisation + SHA256/BLAKE3

4. Upsert into `aideon_effective_schema_cache` (PK includes hash).
5. Optionally mark “latest schema hash” per type for fast lookup (see 24.11).

**Cycle detection:**

- detect and error if cycle introduced; mark job failed and expose validation report.

#### 24.8.2 Analytics projection maintenance processor

**Purpose:** ensure analytics modules have clean adjacency and optionally light aggregates.

**Triggers:**

- edge create/tombstone ops
- edge existence interval ops (if analytics queries need “current active edges” views)
- schema changes affecting relationship semantics (optional)

**Outputs:**

- `aideon_graph_projection_edges` (already maintained synchronously for create/tombstone)
- optional derived tables (see 24.9)

**Consistency mode:**

- projection row creation/update is sync-in-tx (Category A)
- derived aggregates are async (Category B/C)

#### 24.8.3 Integrity and connectivity processor (recommended)

**Purpose:** provide fast signals that the EA repository is sufficiently connected and directionally consistent to support PageRank and similar analyses.

**Triggers:**

- periodic scheduled job
- bulk ingest completion markers
- schema change events (optional)

**Outputs (derived):**

- `aideon_integrity_reports` / `aideon_integrity_metrics` (see 24.10)

**Checks (examples):**

- disconnected components count
- missing required relationship patterns (as defined by metamodel rules supplied by Praxis)
- illegal shortcuts (e.g., capability linked directly to physical app if rules forbid)
- directionality compliance percentage per edge type

### 24.9 Optional derived tables for analytics readiness

These tables are derived and can be rebuilt at any time. They improve performance for analytics modules and dashboards.

#### 24.9.1 `aideon_graph_degree_stats`

- `partition_id` (PK part)
- `scenario_id` (PK part, nullable or separate baseline row)
- `as_of_valid_time` (PK part, nullable for “current”)
- `entity_id` (PK part)
- `out_degree` (int)
- `in_degree` (int)
- `computed_asserted_at` (i64)
  Index:
- `(partition_id, scenario_id, as_of_valid_time, out_degree)`
- `(partition_id, scenario_id, as_of_valid_time, in_degree)`

Computed incrementally from `aideon_graph_projection_edges` filtered by existence at time, or computed for “current” only.

#### 24.9.2 `aideon_graph_edge_type_counts`

- `partition_id`, `scenario_id`, `edge_type_id`, `count`, `computed_asserted_at`
  Index:
- `(partition_id, scenario_id, edge_type_id)`

### 24.10 Integrity reporting tables (recommended)

#### `aideon_integrity_runs`

- `partition_id` (PK part)
- `run_id` (PK part)
- `scenario_id` (nullable)
- `as_of_valid_time` (nullable)
- `as_of_asserted_at` (nullable)
- `params_json` (text)
- `created_asserted_at` (i64)

#### `aideon_integrity_findings`

- `partition_id` (PK part)
- `run_id` (PK part)
- `finding_type` (text)
- `severity` (tiny int)
- `subject_entity_id` (nullable)
- `details_json` (text)

### 24.11 “Latest pointers” for derived artefacts (portable)

To avoid scanning cache tables to find the latest schema hash or latest integrity run, maintain pointer tables updated transactionally.

#### 24.11.1 `aideon_type_schema_head`

- `partition_id` (PK part)
- `type_id` (PK part)
- `schema_version_hash` (text)
- `updated_asserted_at` (i64)

Updated when schema compile job succeeds. Reads can fetch head then load cache blob by `(partition, type, hash)`.

#### 24.11.2 `aideon_integrity_head`

- `partition_id` (PK part)
- `scenario_id` (PK part, nullable)
- `run_id` (text)
- `updated_asserted_at` (i64)

### 24.12 Triggered processing API (Rust)

Mneme exposes explicit controls for processors.

```rust
pub struct TriggerProcessingInput {
  pub partition: PartitionId,
  pub scenario_id: Option<ScenarioId>,
  pub reason: String,
}

pub struct RunWorkerInput {
  pub max_jobs: u32,
  pub lease_millis: u64,
}

pub trait MnemeProcessingApi {
  // enqueue a “rebuild/refresh” job, used after bulk import or admin action
  fn trigger_rebuild_effective_schema(&self, input: TriggerProcessingInput) -> Result<(), MnemeError>;
  fn trigger_refresh_integrity(&self, input: TriggerProcessingInput) -> Result<(), MnemeError>;
  fn trigger_refresh_analytics_projections(&self, input: TriggerProcessingInput) -> Result<(), MnemeError>;

  // worker loop entrypoint for embedding in the host application
  fn run_processing_worker(&self, input: RunWorkerInput) -> Result<u32 /*jobs_processed*/, MnemeError>;

  // observability
  fn list_jobs(&self, partition: PartitionId, status: Option<u8>, limit: u32) -> Result<Vec<JobSummary>, MnemeError>;
}
```

### 24.13 Triggered processing API (TypeScript)

```ts
export interface MnemeProcessingApi {
  triggerRebuildEffectiveSchema(input: {
    partitionId: string;
    scenarioId?: string;
    reason: string;
  }): Promise<void>;
  triggerRefreshIntegrity(input: {
    partitionId: string;
    scenarioId?: string;
    reason: string;
  }): Promise<void>;
  triggerRefreshAnalyticsProjections(input: {
    partitionId: string;
    scenarioId?: string;
    reason: string;
  }): Promise<void>;

  runProcessingWorker(input: {
    maxJobs: number;
    leaseMillis: number;
  }): Promise<{ jobsProcessed: number }>;

  listJobs(input: { partitionId: string; status?: number; limit: number }): Promise<
    Array<{
      jobId: string;
      jobType: string;
      status: number;
      priority: number;
      attempts: number;
      updatedAssertedAt: string;
    }>
  >;
}
```

### 24.14 Real-time processing guarantees

Mneme supports two real-time modes:

#### 24.14.1 Strong real-time (synchronous)

For derived artefacts required for immediate reads (Category A), Mneme updates them in the write transaction:

- projection edges
- indexed field rows
- entity timestamps

Guarantee: after write commit, subsequent reads see these artefacts.

#### 24.14.2 Soft real-time (asynchronous)

For compilation and analytics prep (Category B):

- Mneme enqueues jobs at commit time.
- A worker processes them with bounded latency.
- Host application can run the worker:
  - in-process background task (desktop/server)
  - or periodically (CLI / batch)

Guarantee: eventual completion; progress observable via job tables and APIs.

### 24.15 Bulk ingest behaviour

During bulk import, Mneme must avoid per-op expensive processing.
Mechanism:

- a “bulk mode” flag in write options:
  - `WriteOptions { bulk_mode: bool }`

- In bulk mode:
  - only Category A derived artefacts are updated
  - Category B jobs are coalesced (dedupe keys)
  - a single “rebuild schema” and “refresh integrity” job is scheduled at end

### 24.16 Failure handling and retries

- Jobs retry with exponential backoff (implemented in code; store `next_run_after` in jobs table if needed).
- After `max_attempts`, job status becomes failed and is surfaced via API.
- Failures in derived artefacts never corrupt authoritative data.

Add optional columns:

- `next_run_after` (i64)
- `last_error` (text)

Indexes:

- `(partition_id, status, next_run_after, priority)` for runnable selection

### 24.17 Observability and metrics (portable)

Mneme exposes:

- job queue state (pending/running/failed)
- last successful schema compile per type
- last integrity run
- last analytics projection update time per partition

These are accessible via pointer tables and `MnemeProcessingApi`.

### 24.18 Security and tenancy

All triggered processing is scoped by `partition_id` and optional `scenario_id`. Workers must never process jobs across partitions without explicit configuration.

### 24.19 Interactions with analytics modules (contract)

Analytics modules must:

- obtain adjacency via `MnemeAnalyticsApi.getProjectionEdges`
- optionally subscribe/poll for “projection refreshed” events via:
  - job completion observation
  - head pointers (`aideon_integrity_head`, `aideon_type_schema_head`)

- never access database directly

Optional event stream (in-process):

- Mneme can emit callbacks on job completion for real-time UI updates.

---

## 25. Concurrency, isolation, and consistency contracts

### 25.1 Write serialisation and conflict policy boundaries

Mneme guarantees:

- All single-API-call writes are atomic.
- When multiple writers update the same `(entity_id, field_id)` for overlapping valid intervals, Mneme does **not** attempt to prevent overlaps; it resolves them deterministically at read time using merge policy + precedence.

Optional safeguards (configurable per partition):

- **Overlap warnings**: record a finding when overlapping intervals are introduced for fields with `LWW` policy.
- **Overlap rejection**: reject overlapping intervals for selected fields (requires schema flag `disallow_overlap=true` in `aideon_fields` or `aideon_type_fields`).

### 25.2 Isolation level requirements

Mneme supports the DB’s default isolation (SQLite SERIALIZABLE-ish, Postgres READ COMMITTED by default, MySQL REPEATABLE READ by default). Mneme’s correctness does not depend on stronger isolation because:

- it uses append-only facts + deterministic resolution
- it avoids read-modify-write dependencies where possible

Where read-modify-write is unavoidable (e.g., job leasing), Mneme uses transactional updates and optimistic checks.

### 25.3 Lease safety (jobs)

Job claiming must avoid double execution:

- claim uses a transaction and checks:
  - status is pending OR lease expired
  - then sets running + lease_expires_at

- if the DB cannot guarantee strict row locks (SQLite), rely on update count checks and retry loops.

---

## 26. Data retention, compaction, and archival

### 26.1 Retention policy primitives

Because facts and ops are append-only, Mneme must support configurable retention:

- retain all ops forever (audit-grade)
- retain facts forever, but compact for performance
- retain recent detailed history, archive older history

Mneme exposes:

```rust
pub struct RetentionPolicy {
  pub keep_ops_days: Option<u32>,
  pub keep_facts_days: Option<u32>,
  pub keep_failed_jobs_days: Option<u32>,
  pub keep_pagerank_runs_days: Option<u32>,
}
```

### 26.2 Compaction jobs (derived)

Compaction does not delete authoritative meaning; it reduces redundant representations:

- merge adjacent intervals with same value, layer, and provenance class (optional)
- prune superseded facts for `LWW` fields where older facts can never win again
- rebuild index tables from facts (if drift detected)

Tables affected:

- `aideon_prop_fact_*` (compaction)
- `aideon_idx_field_*` (rebuild)
- `aideon_edge_exists_facts` (compaction)

All compaction runs are implemented as jobs (Category C) and are safe to interrupt and resume.

### 26.3 Archival export (portable)

Mneme provides an export format independent of DB:

- op-log export is the canonical archive (binary envelopes)
- optional snapshot export for quick restore

APIs:

- `export_ops` already covers this; add:
  - `export_partition_snapshot` (derived, optional)

---

## 27. Access control hooks (storage-level support)

Mneme does not enforce business ACLs by default, but it must support them without schema redesign.

### 27.1 ACL-ready columns

Add optional columns to `aideon_entities`:

- `acl_group_id` (nullable text/id)
- `owner_actor_id` (nullable)
- `visibility` (tiny int)

These are authoritative properties at the storage layer, but interpreted by higher layers.

### 27.2 Query filtering hooks

Mneme APIs accept an optional “security context” that is applied in code:

- deny returning entity IDs not visible to the caller
- for traversal, filter edges by endpoint visibility rules

This avoids pushing ACL logic into SQL while keeping portability.

---

## 28. Subscriptions and change feeds (real-time UI and services)

### 28.1 Purpose

Provide a structured change stream without exposing SQL:

- UI can subscribe to entity changes or query result changes
- analytics modules can trigger re-runs when relevant changes occur

### 28.2 Change feed table (portable)

If in-process events are not enough, persist a feed:

#### `aideon_change_feed`

- `partition_id`
- `sequence` (auto-increment per partition; emulate in SQLite via rowid table)
- `op_id`
- `asserted_at`
- `entity_id` (nullable)
- `change_kind` (tiny int)
- `payload` (optional small json)
  PK: `(partition_id, sequence)`

Mneme appends to this in the same transaction as writes (Category A).

### 28.3 Subscription API

Rust:

```rust
fn subscribe_partition(partition: PartitionId, from_sequence: Option<i64>) -> Receiver<ChangeEvent>;
```

TS:

- polling API:
  - `getChangesSince(sequence)` returns events + new sequence

This is portable and works even in embedded SQLite without a server.

---

## 29. Canonical encoding and normalisation standards

### 29.1 Value normalisation rules

To ensure cross-platform identical behaviour:

- string normalisation for indexed strings:
  - trim
  - unicode NFC normalisation (if feasible in Rust)
  - case-fold (locale-independent)

- time values always stored as epoch micros UTC
- UUID textual encoding canonical (lowercase, hyphenated)

### 29.2 Hashing and schema versioning

Schema version hashes must be:

- stable across platforms and architectures
- based on canonical serialisation of relevant rows
- produced in code (e.g., BLAKE3) and stored as hex/base64 text

---

## 30. Error model and diagnostics

### 30.1 Error classes

- `ValidationError` (schema or type mismatch)
- `NotFoundError` (entity/type/field)
- `ConflictError` (optional strict overlap enforcement)
- `StorageError` (DB errors)
- `ProcessingError` (job failures)
- `SyncError` (op ingestion issues)

TS errors must preserve:

- error code
- human-readable message
- structured details (entity_id, field_id, etc.)

### 30.2 Diagnostics APIs

- `getIntegrityHead`, `getLastSchemaCompile`, `listFailedJobs`
- `explainResolution(entity, field, T)` (optional debug-only):
  - returns which facts were considered and why winner selected
  - critical for trust in time travel and conflict resolution

---

## 31. Security and safety constraints

### 31.1 Input hardening

- validate payload sizes (prevent oversized blobs in ops)
- limit MV results size (cap values returned; expose “more available” marker)
- limit traversal fanout per call (default max, configurable)

### 31.2 Denial-of-service guardrails

- bounded queries:
  - always require `limit` on list/traverse/projection fetch

- job queue backpressure:
  - maximum pending jobs per partition
  - dedupe keys mandatory for high-frequency triggers

---

## 32. Compatibility and upgrade guarantees

### 32.1 Forward-only migrations

- Mneme supports forward migrations only.
- Data export/import via op-log provides downgrade path if needed.

### 32.2 Stable API contracts

- Rust and TS APIs are versioned.
- Breaking schema changes require a migration plus an API minor/major bump.

---

## 33. Suggested minimal additions to the schema to support 25–32

If implementing these sections, add the following tables/columns:

Tables:

- `aideon_jobs` (+ optional `aideon_job_events`)
- `aideon_type_schema_head`
- `aideon_integrity_runs`, `aideon_integrity_findings`, `aideon_integrity_head`
- `aideon_change_feed` (optional but recommended)

Columns:

- `valid_bucket` on time-valid fact tables (existence + prop facts + indexes)
- job queue: `dedupe_key`, `next_run_after`, `last_error`

All are implementable with SeaORM migrations and portable indexes.

---

## 34. Portable export and import (flat-file, replayable)

### 34.1 Purpose

Mneme must support **lossless export and import** of a partition (or scenario) to and from **flat files**, so that data can be:

- backed up independently of the underlying RDBMS
- moved between environments (SQLite → Postgres, Postgres → MySQL, etc.)
- replayed into an empty database to reconstruct identical state
- diffed, inspected, or versioned externally if required

The design must:

- avoid DB-specific dump formats
- avoid raw SQL
- rely only on Mneme APIs and canonical encodings
- preserve determinism and provenance

---

## 34.2 Export model: op-log as the canonical truth

### 34.2.1 Canonical export unit

The **operation log** (`aideon_ops` + dependencies) is the **authoritative export format**.

Everything else (entities, facts, indexes, projections, caches) is **derivable**.

Therefore:

- **Export = ordered stream of ops + minimal metadata**
- **Import = replay ops into empty Mneme store**

This guarantees:

- portability across DB backends
- forward compatibility with new derived artefacts
- deterministic reconstruction

---

## 34.3 Export formats (flat file)

Mneme supports two flat-file formats:

### 34.3.1 Streaming NDJSON (recommended default)

- One JSON object per line
- Easy to stream, append, compress, and inspect
- Suitable for very large partitions

File extension: `.mneme.ndjson` (optionally `.gz`)

#### Record types

Each line has a `record_type` field.

##### Header (first record)

```json
{
  "record_type": "header",
  "format_version": "1.0",
  "exported_at_asserted": "HLC_STRING",
  "partition_id": "UUID",
  "scenario_id": null,
  "mneme_version": "x.y.z",
  "options": {
    "include_schema": true,
    "include_ops": true
  }
}
```

##### Operation record

```json
{
  "record_type": "op",
  "partition_id": "UUID",
  "scenario_id": null,
  "op_id": "UUID",
  "actor_id": "UUID",
  "asserted_at": "HLC_STRING",
  "op_type": 12,
  "payload_base64": "…",
  "deps": ["UUID", "UUID"]
}
```

##### Footer (optional)

```json
{
  "record_type": "footer",
  "op_count": 1234567,
  "checksum": "BLAKE3_HASH"
}
```

### 34.3.2 Binary chunked format (optional)

For performance-critical cases:

- framed binary records
- length-prefixed
- CBOR or MessagePack payloads

Still logically:

- header
- op stream
- footer

This is optional; NDJSON is sufficient for v1.

---

## 34.4 Export scope options

Mneme export APIs allow precise control:

```rust
pub struct ExportOptions {
  pub partition_id: PartitionId,
  pub scenario_id: Option<ScenarioId>,
  pub since_asserted_at: Option<AssertedTime>,
  pub until_asserted_at: Option<AssertedTime>,
  pub include_schema: bool,     // usually true
  pub include_data_ops: bool,   // usually true
  pub include_scenarios: bool,  // export scenario overlays
}
```

Supported export scopes:

- full partition (baseline only)
- single scenario (overlay only)
- baseline + scenario
- incremental export (“since last sync”)
- schema-only export (metamodel migration)

---

## 34.5 Import model: replay into empty or existing store

### 34.5.1 Import guarantees

Import must be:

- **idempotent** (re-importing same file does not change result)
- **order-robust** (ops may arrive out of order, subject to deps)
- **transactional in batches** (bounded memory)

### 34.5.2 Import phases

#### Phase 1: Preflight

- validate header:
  - format_version compatible
  - partition_id matches or remapped

- optional dry-run validation:
  - schema ops precede data ops (recommended but not strictly required)
  - detect missing dependencies

#### Phase 2: Schema replay (if included)

- ingest metamodel ops first
- compile effective schemas as jobs (not blocking import)

#### Phase 3: Data replay

- ingest ops in asserted_at order if possible
- otherwise ingest in file order with dep checks
- use Mneme `ingest_ops` API (same as sync)

#### Phase 4: Post-import processing

- enqueue:
  - effective schema rebuild
  - analytics projection refresh
  - integrity scan

- do **not** export/import derived artefacts

---

## 34.6 Empty-database replay (primary use case)

To replay into an empty database:

1. Run Mneme migrations to create schema.
2. Call import API with `allow_create_partition=true`.
3. Replay ops.
4. Run processing workers until queues empty.

Result:

- identical logical state
- identical resolution at all valid times
- potentially different physical row ordering (irrelevant)

---

## 34.7 Import into non-empty database (advanced)

Supported but restricted.

Rules:

- ops are deduped by `(partition_id, op_id)`
- if partition exists:
  - imported ops must not violate schema compatibility
  - schema conflicts become validation errors or warnings

- optional remapping:
  - partition_id remap
  - scenario_id remap
  - actor_id remap

```rust
pub struct ImportOptions {
  pub target_partition: PartitionId,
  pub scenario_id: Option<ScenarioId>,
  pub allow_partition_create: bool,
  pub remap_actor_ids: HashMap<ActorId, ActorId>,
  pub strict_schema: bool,
}
```

---

## 34.8 Streaming import/export APIs

### 34.8.1 Rust APIs

```rust
pub trait MnemeExportApi {
  fn export_ops_stream(
    &self,
    options: ExportOptions,
  ) -> Result<Box<dyn Iterator<Item = ExportRecord>>, MnemeError>;
}

pub trait MnemeImportApi {
  fn import_ops_stream<I>(
    &self,
    options: ImportOptions,
    records: I,
  ) -> Result<ImportReport, MnemeError>
  where
    I: Iterator<Item = ExportRecord>;
}
```

### 34.8.2 TypeScript APIs

```ts
export interface MnemeExportApi {
  exportOps(options: {
    partitionId: string;
    scenarioId?: string;
    sinceAssertedAt?: string;
    untilAssertedAt?: string;
  }): AsyncIterable<{
    recordType: 'header' | 'op' | 'footer';
    data: any;
  }>;
}

export interface MnemeImportApi {
  importOps(
    options: {
      partitionId: string;
      scenarioId?: string;
      allowPartitionCreate?: boolean;
      strictSchema?: boolean;
    },
    records: AsyncIterable<any>,
  ): Promise<{
    opsImported: number;
    opsSkipped: number;
    errors: number;
  }>;
}
```

---

## 34.9 Checksums and integrity verification

### 34.9.1 Export integrity

- footer checksum = hash of all op records in order
- algorithm: BLAKE3
- stored as hex/base64

### 34.9.2 Import verification

- optional checksum verification
- optional per-op hash verification (future)

If checksum mismatch:

- import aborts (strict)
- or continues with warning (non-strict)

---

## 34.10 Performance characteristics

### Export

- streaming, O(1) memory
- uses cursor over `aideon_ops` ordered by `asserted_at`
- compression handled externally (gzip/zstd)

### Import

- batch size configurable (e.g. 1k–10k ops per transaction)
- bounded memory
- backpressure via job queue if schema/processing jobs pile up

---

## 34.11 What is explicitly _not_ exported

To keep exports stable and portable, Mneme **does not export**:

- derived tables:
  - `aideon_effective_schema_cache`
  - `aideon_graph_projection_edges`
  - `aideon_pagerank_*`
  - integrity reports
  - job queue

- DB-specific metadata (indexes are recreated via migrations)

These are always rebuilt after import.

---

## 34.12 Determinism guarantee

Given:

- same export file
- same Mneme version (or forward-compatible)
- same replay order

Mneme guarantees:

- identical resolution of state at any valid time
- identical PageRank inputs (after projections rebuilt)
- identical conflict outcomes

This makes flat-file export/import suitable for:

- disaster recovery
- CI test fixtures
- reproducible analytics experiments
- long-term archival

---

## 34.13 Recommended usage patterns

### Backup

- nightly NDJSON export per partition
- store compressed
- periodically test restore into empty DB

### Environment promotion

- export from dev → import into test
- schema validation in strict mode

### Analytics sandbox

- export baseline partition
- import into isolated DB
- run heavy PageRank and simulations without affecting prod

---

## 35. Snapshotting and fast restore (hybrid export/import)

### 35.1 Purpose

Op-log replay is correct but can be slow for very large partitions. Mneme supports optional **snapshots** to accelerate restore while preserving portability.

### 35.2 Snapshot design

A snapshot is a **portable logical state capture** at a chosen `as_of_asserted_at` and optionally `as_of_valid_time`:

- includes authoritative state tables only (entities + facts)
- excludes derived artefacts (indexes, projections, caches) unless explicitly requested as accelerators

Snapshot is still database-agnostic: it is exported as flat-file records.

### 35.3 Snapshot record types

Add record types alongside ops:

- `snapshot_header`
- `snapshot_entity`
- `snapshot_edge`
- `snapshot_fact_*` (typed)
- `snapshot_footer`

### 35.4 Import using snapshot + ops tail

Restore algorithm:

1. Import snapshot into empty DB in bulk (fast inserts).
2. Replay ops whose `asserted_at > snapshot_as_of`.
3. Rebuild derived artefacts/jobs.

Guarantee: identical state as full replay.

### 35.5 Snapshot API

Rust:

```rust
pub struct SnapshotOptions {
  pub partition_id: PartitionId,
  pub scenario_id: Option<ScenarioId>,
  pub as_of_asserted_at: AssertedTime,
  pub include_facts: bool,
  pub include_entities: bool,
}
pub trait MnemeSnapshotApi {
  fn export_snapshot_stream(&self, opts: SnapshotOptions) -> Result<Box<dyn Iterator<Item=ExportRecord>>, MnemeError>;
  fn import_snapshot_stream<I>(&self, opts: ImportOptions, records: I) -> Result<(), MnemeError>
  where I: Iterator<Item=ExportRecord>;
}
```

---

## 36. Database vacuum, pruning, and maintenance (portable)

### 36.1 Maintenance tasks (jobs)

Mneme schedules and runs maintenance via the job system:

- compaction (Section 26)
- snapshot generation (optional)
- stale job cleanup
- old derived artefact cleanup
- (SQLite) VACUUM-equivalent is not portable via SeaORM without raw SQL; Mneme therefore:
  - provides a _hook_ for host to call DB-specific maintenance where allowed
  - remains correct without it

### 36.2 Cleanup policies

Derived artefacts cleanup:

- old PageRank runs beyond retention
- old integrity runs
- old schema caches (keep last N per type)

Authoritative data pruning (only if configured):

- prune ops/facts older than X days only if:
  - snapshot exists after the cutoff, or
  - audit history is not required

---

## 37. Schema evolution and migrations (metamodel changes over time)

### 37.1 Metamodel versioning

Mneme treats metamodel updates as data ops; however, it also supports “schema generation versioning”:

- each metamodel batch update may include:
  - `metamodel_version` (string)
  - `metamodel_source` (Praxis commit hash, etc.)

Store optional fields in `aideon_ops.payload` or add:

- `aideon_metamodel_versions` table (derived)

### 37.2 Field type changes (migration)

If Praxis changes a field’s `value_type`:

- Mneme enforces explicit migration mode:
  - reject in strict mode unless a migration plan is provided
  - or accept but mark field as “incompatible” and block writes

Optional migration support:

- create new `field_id`, deprecate old field
- provide mapping in application layer
- storage stays stable

---

## 38. Validation framework (rules as data, execution in Mneme)

### 38.1 Rule storage

Mneme can store validation rules from Praxis in a portable form:

- rule applies to:
  - type
  - edge type
  - field
  - relationship patterns (src/dst constraints)

- rule severity: warn/error
- rule expression: template-based, not arbitrary code

Table:

#### `aideon_validation_rules`

- `partition_id` (PK part)
- `rule_id` (PK part)
- `scope_kind` (tiny int)
- `scope_id` (nullable)
- `severity` (tiny int)
- `template_kind` (text)
- `params_json` (text)
- `updated_asserted_at` (i64)

### 38.2 Validation execution

- on write (sync) for cheap checks
- as job (async) for expensive checks
- findings stored in integrity tables

---

## 39. Computed attributes support boundary (storage vs compute)

### 39.1 Storage responsibility

Mneme should store:

- computed field definitions (templates) if required by the metamodel
- computed results as derived artefacts (optional)

However, computed evaluation may be owned by a separate compute module. Mneme must still support it cleanly.

### 39.2 Storage tables (optional)

#### `aideon_computed_rules`

- `partition_id`, `rule_id`, `target_type_id`, `output_field_id`, `template_kind`, `params_json`, `updated_asserted_at`

#### `aideon_computed_cache_*` (typed)

Same pattern as facts, but marked derived:

- `partition_id`, `entity_id`, `field_id`, `valid_from`, `valid_to`, `value_*`, `rule_version_hash`, `computed_asserted_at`

### 39.3 Trigger integration

Computed cache rebuild jobs triggered by:

- changes to dependencies (edges, fields)
- rule changes

---

## 40. Multi-tenant and multi-user concerns

### 40.1 Partition hard isolation

All APIs require `partition_id`. Mneme must:

- refuse operations missing partition
- never join across partitions in internal queries

### 40.2 Actor identity

Actors are used for:

- op provenance
- tie-breaking determinism when `asserted_at` equal
- audit reporting

Actor registration:

- created lazily on first op if not present (configurable)

---

## 41. Large object handling (blobs) and attachment strategy

### 41.1 Inline blobs vs external store

For portability and simplicity, Mneme supports inline blobs in:

- property blob facts
- op payload

But for large binaries, recommend external object storage with references:

- store URI/reference in `aideon_prop_fact_ref` or `aideon_prop_fact_str`
- keep Mneme payload sizes bounded (enforced in API)

### 41.2 Limits

Configurable per partition:

- max op payload size
- max blob property size
- max number of facts per write batch

---

## 42. Pagination and cursors (portable)

### 42.1 Cursor strategy

Offset pagination is not stable at scale. Mneme uses keyset pagination:

- cursor encodes last seen `(sort_key..., entity_id)`
- implemented in SeaORM query builders

### 42.2 Cursor encoding

- base64url JSON of cursor struct
- signed optional (host-provided secret) if exposed outside trusted boundary

---

## 43. Robustness against clock skew

### 43.1 Why it matters

Valid time is user-controlled; asserted time is system-controlled but distributed.

Mneme must:

- generate asserted HLC locally for local writes
- accept asserted time from ingested ops, but normalise monotonicity per actor

### 43.2 HLC implementation contract

- per-partition HLC state persisted:
  - `aideon_hlc_state(partition_id, last_hlc)`

- updated transactionally on writes/ingest

This avoids time going backwards after restart.

---

## 44. Testing and verification extensions

### 44.1 Cross-DB equivalence tests

For a fixed op stream:

- replay into SQLite, Postgres, MySQL
- run identical queries:
  - read entity at time
  - traversal at time
  - projection fetch

- assert identical logical results

### 44.2 Export/import roundtrip tests

- generate random ops
- export NDJSON
- import into empty DB
- assert identical query results

### 44.3 Snapshot hybrid tests

- create snapshot at asserted time A
- restore snapshot + tail ops
- compare to full replay restore

### 44.4 Job coalescing tests

- simulate bulk ingest
- assert job count bounded by dedupe keys
- assert correctness of derived artefacts after worker run

---

## 45. Reference implementation guidance (SeaORM patterns)

### 45.1 Repository pattern

Mneme should structure DB access behind repositories:

- `OpsRepo`, `EntitiesRepo`, `FactsRepo`, `SchemaRepo`, `JobsRepo`, `ProjectionRepo`
  Each repository:
- accepts `&DatabaseConnection` or `&DatabaseTransaction`
- uses only SeaORM query builders

### 45.2 Bulk insert patterns

Use `insert_many` with chunk sizes:

- SQLite: smaller chunks (e.g., 200–500)
- Postgres/MySQL: larger chunks (e.g., 2k–10k)
  Chunk size determined by backend capability flags (no raw SQL).

### 45.3 Upsert patterns

Use SeaORM `OnConflict`:

- ops: do nothing on conflict (idempotent)
- heads/pointers: update on conflict
- jobs: upsert by `(partition_id, job_type, dedupe_key)` where supported via unique index

---

## 46. Documentation artefacts Mneme must ship with

### 46.1 Schema manifest

Generate a machine-readable schema manifest at build time:

- list of `aideon_*` tables
- columns, types, indexes
- migration versions

This helps other teams validate environment correctness without SQL introspection.

### 46.2 API contract docs

- Rust trait docs
- TS type docs
- explicit semantics for time, layers, conflicts, scenarios

### 46.3 Operational playbook

- how to run migrations
- how to run workers
- how to export/import
- recommended retention and compaction schedules

---

## 47. Optional but high-value: deterministic “explain” tooling

### 47.1 Explain resolution

A debug API to explain why a value was selected at time T:

- returns candidate facts considered, their precedence keys, and the winner.

Tables are unchanged; this is a query + formatting feature.

### 47.2 Explain traversal

Explain why an edge appears in traversal at time T:

- which existence fact made it active
- whether entity tombstone affected it

This materially improves trust in time travel and analytics.

---

## 48. Summary of comprehensive scope achieved (25–48)

With Sections 25–48, Mneme now covers:

- concurrency and isolation contracts
- retention and compaction
- portable export/import with snapshots
- triggered processing pipelines
- validation framework
- computed attribute storage hooks
- change feeds and subscriptions
- robust pagination
- clock skew resilience
- cross-DB equivalence testing
- SeaORM implementation guidance
- explain/debug tooling
