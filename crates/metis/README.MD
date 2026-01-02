# Metis Analytics – Aideon Suite module

## Purpose

Metis Analytics is the analytics engine for Aideon Suite. It implements graph algorithms and
financial analytics (e.g. shortest path, centrality, impact, TCO) over the time-first digital twin.

## Responsibilities

- Implement analytics jobs defined in the worker contracts (shortest path, centrality, impact, TCO).
- Operate on snapshots and commit history supplied by Praxis Engine/Mneme Core.
- Surface results via typed traits for host/worker orchestration.

## Relationships

- **Depends on:** Praxis Engine, Mneme Core.
- **Used by:** Aideon Host, Continuum Orchestrator, and downstream reporting surfaces.

## Running and testing

- Rust tests (crate only): `cargo test -p aideon_metis`
- Workspace checks: `pnpm run host:lint && pnpm run host:check`

## Design and architecture

Metis honours the RPC and job contracts described in `ARCHITECTURE-BOUNDARY.MD` and `docs/DESIGN.MD`.
As algorithms land, extend `crates/metis/DESIGN.MD` to capture structures, SLOs,
and test datasets.
