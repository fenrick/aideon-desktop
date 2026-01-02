# Chrona Visualisation – Aideon Suite module

## Purpose

Chrona Visualisation provides temporal views over the digital twin. It implements helpers for
`state_at`, `diff`, and future topology/time-based visualisations consumed by Praxis Canvas and
other clients.

## Responsibilities

- Implement temporal summarisation APIs over Praxis Engine snapshots.
- Provide data structures tailored to time sliders, timelines, and diff views.
- Work with Aideon Host to expose Tauri commands for temporal queries.

## Relationships

- **Depends on:** Praxis Engine and Mneme Core for raw state and commits.
- **Used by:** Aideon Host, Praxis Canvas (via IPC), analytics layers that need temporal slices.

## Running and testing

- Rust tests (crate only): `cargo test -p aideon_chrona`
- Workspace checks: `pnpm run host:lint && pnpm run host:check`

## Design and architecture

Chrona follows the time-first principles captured in `ARCHITECTURE-BOUNDARY.MD` and `docs/DESIGN.MD`
while focusing purely on temporal views. As this crate evolves, document internal modules and
performance constraints in `crates/chrona/DESIGN.MD`.
