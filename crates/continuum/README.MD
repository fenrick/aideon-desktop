# Continuum Orchestrator – Aideon Suite module

## Purpose

Continuum Orchestrator coordinates connectors, schedules, and snapshot/layout persistence for Aideon
Suite. It is responsible for automation-centric flows such as CMDB syncs and scheduled freshness
checks.

## Responsibilities

- Provide traits and implementations for scheduling and orchestration.
- Manage snapshot and layout persistence for canvas-related workflows.
- Integrate with external systems (e.g. CMDB) via adapters while respecting security boundaries.

## Relationships

- **Depends on:** Praxis Engine, Mneme Core, external connector crates (as they appear).
- **Used by:** Aideon Host, worker processes, and potential server mode.

## Running and testing

- Rust tests (crate only): `cargo test -p aideon_continuum`
- Workspace checks: `pnpm run host:lint && pnpm run host:check`

## Design and architecture

Continuum follows the adapter and orchestration guidance in `ARCHITECTURE-BOUNDARY.MD`. As more
connectors and schedules are added, capture internal design in
`crates/continuum/DESIGN.MD`.
