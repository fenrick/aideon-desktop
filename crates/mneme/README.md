# Mneme Core – Aideon Suite module

## Purpose

Mneme Core is the persistence layer for Aideon Suite. It provides the op-log, bi-temporal fact
storage, schema-as-data, and graph projections described in `crates/mneme/DESIGN.md`.
SQLite is the first implementation; other RDBMS backends are expected to follow the same logical
schema and APIs.

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

- Rust tests (crate only): `cargo test -p aideon_mneme`
- Workspace checks: `pnpm run host:lint && pnpm run host:check`

## Design and architecture

`crates/mneme/DESIGN.md` is the authoritative design spec. Update it when storage semantics or
APIs change. Keep `docs/storage/sqlite.md` aligned with the current schema and migration strategy.
