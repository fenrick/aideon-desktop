# Mneme Core – Design

## Purpose & scope

Mneme Core owns persistence for the Praxis twin: commits, refs, snapshots, and related analytics
artifacts. It supplies storage traits and migrations used by engine/analytics crates.

## Allowed dependencies / frameworks

- Rust 2024 with workspace defaults.
- `tokio` for async when required, `serde`/`serde_json` for config/DTOs, `thiserror` for errors,
  `tracing` + `log` facade for observability.
- DB layer: SQLite via SeaORM/`sqlx`-style backends as configured; keep DB drivers isolated here.

## Anti-goals

- No renderer/Tauri/UI code.
- No analytics or business logic; this crate is persistence-only.
- Avoid duplicating migration logic in dependants; keep schema changes here.

## Public surface

- Storage traits for commits/refs/snapshots and helper types for migrations.
- DB adapter implementations (SQLite today) hidden behind those traits.
- Utilities for connection management and schema versioning.

## Evergreen notes

- Migrate any direct DB calls in other crates into Mneme traits.
- Phase out legacy or hand-written SQL in favour of the chosen ORM/query layer while keeping
  schemas forward-only.
