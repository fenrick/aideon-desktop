# SQLite Storage Layer (Mneme)

## Purpose

Provide a reference for the SQLite storage layer used by Mneme Core in desktop mode. The
authoritative design is `crates/mneme/DESIGN.md`; this document highlights SQLite-specific
implementation details and portability considerations for the SeaORM-backed store.

## Schema overview (Mneme v1)

Mneme stores:

- **Operation log**: `aideon_ops`, `aideon_op_deps`
- **Entities and edges**: `aideon_entities`, `aideon_edges`, `aideon_edge_exists_facts`
- **Typed property facts**: `aideon_prop_fact_*` tables
- **Schema metadata**: `aideon_types`, `aideon_fields`, `aideon_type_fields`, `aideon_type_extends`
- **Edge semantics**: `aideon_edge_type_rules`
- **Projection tables**: `aideon_graph_projection_edges`, optional `aideon_pagerank_runs`/
  `aideon_pagerank_scores`
- **Indexed fields**: `aideon_idx_field_*`

SQLite DDL is implemented via SeaORM migrations in `crates/mneme/src/migration`. All IDs are stored
as `TEXT` UUID strings; valid time is stored as `INTEGER` (microseconds), and HLC asserted time is
stored as `INTEGER` (portable i64 encoding).

> Note: Mneme's schema does **not** include per-table `scenario_id` columns. Scenarios are expressed
> via Praxis context and stored as facts/artefacts using Mneme's standard primitives.

## Migration + DDL management

- Migrations are applied at startup by the Mneme store.
- Schema versions are tracked in `aideon_schema_version` (SeaORM also tracks its own migration
  table).
- Keep SQL portable; avoid SQLite-only functions in core logic.

## Portability checklist

1. Stick to `INTEGER`, `TEXT`, `REAL`, `BLOB` columns.
2. Keep IDs and HLC encoded in application code.
3. Use `INSERT ... ON CONFLICT ...` for upserts.
4. Keep JSON usage limited to metadata fields and non-indexed values.

## File locations

Desktop mode stores the database under the Praxis application data directory. The host should
create the directory and open the DB using Mneme configuration (for example, `mneme.json`) and
default to SQLite when not configured.

Example `mneme.json` (SQLite with explicit pool settings):

```json
{
  "database": { "backend": "sqlite", "path": "praxis.sqlite" },
  "pool": {
    "max_connections": 10,
    "min_connections": 1,
    "connect_timeout_ms": 1000,
    "acquire_timeout_ms": 1000,
    "idle_timeout_ms": 60000
  }
}
```

Example `mneme.json` (Postgres/MySQL):

```json
{
  "database": { "backend": "postgres", "url": "postgres://user:pass@host/db" }
}
```

## Future swaps

- **Managed Postgres / MySQL**: Use SeaORM connection URLs in `mneme.json` to point at the target
  RDBMS while keeping Mneme APIs stable.
- **FoundationDB**: Map op log + fact tables to transactional key prefixes, maintaining the same
  resolution rules from Mneme (outside current SeaORM scope).
