# Continuum Orchestrator – Design

## Purpose & scope

Continuum Orchestrator coordinates connectors, schedules, and persistence workflows (e.g. CMDB
ingest, snapshot refresh). It provides the automation plane for Praxis.

## Allowed dependencies / frameworks

- Rust 2024 with workspace defaults.
- `tokio` for async scheduling, `serde`/`serde_json` for configs and job payloads, `thiserror` for
  error types, `tracing` + `log` facade for observability.
- Orchestration interfaces depend on `praxis` and `mneme`; connector
  integrations live behind traits/adapters.

## Anti-goals

- No renderer/UI or Tauri bindings.
- No direct DB coupling beyond Mneme traits; avoid embedding SQL here.
- No bespoke schedulers or thread pools when Tokio timers suffice.

## Public surface

- Traits for scheduling and orchestration of connector jobs and snapshot/layout persistence.
- Adapter interfaces for external systems (CMDB, file imports) with clear contract types.
- Helper functions to compose engine + persistence flows for the host/worker.

## Evergreen notes

- Replace any legacy shell-scripted scheduling with Tokio-driven adapters.
- Converge ad-hoc connector code onto the shared adapter interfaces before adding new backends.
