# Praxis Engine – Aideon Suite module

## Purpose

Praxis Engine is the core time-aware graph engine for Aideon Suite. It owns commits, branches,
snapshots, scenarios, and schema validation for the digital twin.

## Responsibilities

- Maintain the commit graph and snapshot materialisation for the twin.
- Enforce the meta-model and relationship constraints.
- Implement `state_at`, `diff`, and related temporal operations used by Chrona and the canvas.
- Expose traits and types consumed by Aideon Host and other engine crates.

## Relationships

- **Depends on:** Mneme Core for persistence, shared DTOs for commits/refs/snapshots.
- **Used by:** Aideon Host, Chrona Visualisation, Metis Analytics, Continuum Orchestrator.

## Facade responsibilities (merged)

Praxis now exposes the higher-level façade surface directly.

### Responsibilities

- Wrap lower-level engine operations (commits, snapshots, queries) in cohesive use-case APIs.
- Coordinate calls into Chrona/Metis/Continuum where composite operations are needed.
- Provide a stable boundary for future server mode without exposing storage internals.

### Relationships

- **Depends on:** Praxis Engine, Mneme Core, and other engine crates as they mature.
- **Used by:** Aideon Host and future server/CLI entry points.

### Design notes

- No UI/Tauri code and no renderer-facing APIs.
- No persistence internals; storage stays behind Mneme traits.
- Avoid duplicating logic that belongs in Engine/Chrona/Metis—compose them instead.
- Fold host-side orchestration fragments here to keep IPC handlers thin.

## Running and testing

- Rust tests (crate only): `cargo test -p aideon_praxis`
- Workspace Rust checks: `pnpm run host:lint && pnpm run host:check`

## Design and architecture

Praxis Engine follows the time-first commit model described in `Architecture-Boundary.md` and
`docs/DESIGN.md`. Internal modules (graph, commits, meta-model, importer) are outlined in
`crates/praxis/DESIGN.md`.
