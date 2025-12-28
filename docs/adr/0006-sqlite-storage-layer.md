# ADR-0006: SQLite Storage Layer for Commits/Refs/Snapshots

## Status

Superseded by `crates/mneme/DESIGN.md` (Mneme storage redesign, 2025-12-28). SQLite remains the
default backend, but the schema and APIs are now the Mneme fact/ops model, implemented via SeaORM.

## Context

Aideon Suite needs an embedded, durable store for commits, refs, and snapshots in desktop mode that:

- Works well with Tauri and local-first usage.
- Can later be migrated to a server-grade backend (e.g., PostgreSQL-family, FoundationDB).
- Keeps schema and DTOs aligned with Mneme Core and Praxis Engine.

Early experiments stored state in memory or ad-hoc files; we need a more principled storage story.

## Decision

Use **SQLite 3** as the default storage engine for Mneme Core in desktop builds:

- Store Mneme ops/facts/schema/projections using portable `TEXT`/`INTEGER`/`BLOB` columns and
  SeaORM migrations (no raw SQL in migrations or runtime paths).
- Use portable upserts and indexes so the schema can be mapped to PostgreSQL/MySQL backends later.

Mneme Core (`crates/mneme`) is responsible for migrations and DDL; Praxis Engine and
other crates use only its repository-style APIs.

## Consequences

- Desktop builds gain an ACID-compliant data store with low operational overhead.
- Server/cloud deployments can swap in different `CommitStore` implementations (e.g., Postgres) by
  recreating the schema and reusing DTO serialisation.
- Schema-specific tuning and migrations must stay in Mneme Core; other crates should not embed raw
  SQL beyond carefully reviewed queries.

## References

- `docs/storage/sqlite.md` – schema and migration details
- `crates/mneme/src/store.rs` – SeaORM-backed Mneme store
- `crates/mneme/src/migration` – schema and migration definitions
- `docs/DESIGN.md` – high-level data/integration overview
