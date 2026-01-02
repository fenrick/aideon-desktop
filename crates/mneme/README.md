# Mneme Core – Aideon Suite module

## Purpose

Mneme Core is the persistence layer for Aideon Suite. It provides the op-log, bi-temporal fact
storage, schema-as-data, and graph projections described in `crates/mneme/DESIGN.md`.
SQLite is the first implementation; other RDBMS backends are expected to follow the same logical
schema and APIs.

## Package layout

- `aideon_mneme_core`: shared types + API traits.
- `aideon_mneme_store`: SeaORM-backed storage engine, migrations, and workers.
- `aideon_mneme`: façade crate re-exporting core + store for downstream consumers.

## Responsibilities

- Own the only SQL/RDBMS access layer in the suite.
- Provide storage APIs for entities, edge existence, typed property facts, schema metadata, and
  analytics projections (PageRank-ready adjacency).
- Maintain the operation log used for sync and deterministic replication.
- Run DDL and migrations for the storage engine.
- Support scenario overlays via `scenario_id` scoped facts and reads.

## Relationships

- **Depends on:** SQLite/SeaORM (or other persistence libs as they are added).
- **Used by:** Praxis Engine, Metis Analytics, Continuum Orchestrator, Praxis Facade.

## Running and testing

- Rust tests (store): `cargo test -p aideon_mneme_store`
- Workspace checks: `pnpm run host:lint && pnpm run host:check`

## Operational playbook

### Migrations

- `MnemeStore::connect` runs migrations automatically via the SeaORM migrator.
- For manual runs in tooling, invoke `Migrator::up` with the active database connection.

### Workers and processing

- Use `trigger_rebuild_effective_schema`, `trigger_refresh_integrity`, and
  `trigger_refresh_analytics_projections` to enqueue jobs.
- Call `run_processing_worker` to lease and execute jobs inside the host process.

### Export and import

- Use `export_ops` to stream the op-log (canonical sync format).
- Use `export_snapshot_stream` / `import_snapshot_stream` for fast restore snapshots.

### Retention and compaction

- Schedule `trigger_retention` daily or weekly with a `RetentionPolicy` to bound op-log, fact, job,
  and PageRank history.
- Schedule `trigger_compaction` after retention windows to rebuild index tables from facts.

## Design and architecture

`crates/mneme/DESIGN.md` is the authoritative design spec. Update it when storage semantics or
APIs change. Keep `docs/storage/SQLITE.md` aligned with the current schema and migration strategy.
