# Praxis Engine – Internal Design

## Overview

Praxis Engine owns the time-first digital twin: commits, branches, snapshots, and schema
validation. It implements the commit model and `state_at`/`diff` semantics described in
`ARCHITECTURE-BOUNDARY.MD`.

## Internal structure

- Commit and branch management modules (append-only DAG).
- Snapshot materialisation and caching.
- Meta-model registry and validation.
- Import/export helpers for baseline datasets.

## Data model and APIs

- Core types: Commit, Branch, Snapshot, ChangeSet, NodeVersion, EdgeVersion.
- Public APIs for `commit`, `state_at`, `diff`, branch operations, and meta-model hydration.
- Works against persistence traits provided by Mneme Core.

## Implementation notes

- Errors: `PraxisError` via `thiserror`, exported as `PraxisResult`.
- Async: `tokio` for tests/integration; engine functions are `async` but keep logic synchronous where
  possible.
- Logging: prefer `tracing`/`log` macros for structured debug; avoid bespoke logging helpers.
- Persistence: go through Mneme `Store` traits (`MemoryStore`, `SqliteDb`); no direct DB drivers in
  this crate.

## Interactions

- Called by Aideon Host to service IPC commands.
- Supplies snapshots and diffs to Chrona Visualisation and Metis Analytics.
- Reads/writes data exclusively through Mneme Core persistence interfaces.

## Facade responsibilities (merged)

- Expose higher-level orchestration APIs so hosts/tools can avoid wiring details.
- Coordinate calls into Chrona/Metis/Continuum for composite workflows.
- Keep storage details behind Mneme traits; avoid direct DB drivers here.

## Constraints and invariants

- Commit history is append-only; no rewrites.
- Schema validation must pass before persistence.
- Diff/merge semantics follow the rules documented in `ARCHITECTURE-BOUNDARY.MD` (e.g. delete wins
  over update by default).

## Patterns in use

- Errors: `PraxisError` + `PraxisResult` (`thiserror`), mapped in host; no custom enums per module.
- Logging: `tracing`/`log` macros with contextual ids; avoid bespoke log wrappers.
- Async: `tokio` + async fns; keep heavy work sync where possible, but expose async APIs for host calls.
- IO/persistence: go through Mneme store traits (`aideon_mneme::store`) and datastore helpers; never reach for raw SQL in this crate.
- Golden examples: temporal ops (`state_at`, `diff`) and merge flow tests (`tests/merge_flow.rs`) show the expected layering (engine core → store → host orchestration).
