# Metis Analytics – Design

## Purpose & scope

Metis implements analytics over the Praxis twin: shortest paths, centrality, impact, TCO and related
jobs defined in the worker contracts.

## Allowed dependencies / frameworks

- Rust 2024 with workspace defaults.
- `tokio` for async where needed, `serde`/`serde_json` for payloads, `thiserror` for error types,
  `tracing` + `log` facade for metrics/observability.
- Graph/metrics helpers (e.g., `petgraph`) are acceptable; keep them behind crate-local abstractions.
- Depends on `aideon_praxis` for snapshots and `aideon_mneme` for persistence traits.

## Anti-goals

- No UI, Tauri, or renderer logic.
- No direct DB access; use Mneme traits to obtain data when persistence is required.
- Avoid bespoke math/graph scaffolding when proven crates exist.

## Public surface

- Traits and structs for analytics jobs (`Analytics.Centrality`, `Analytics.Impact`,
  `Analytics.ShortestPath`, `Finance.TCO`, `Temporal.*` summaries as needed by workers).
- Deterministic helpers to run algorithms against engine snapshots; testable without I/O.

## Evergreen notes

- Migrate any legacy JS/TS prototypes or ad-hoc Rust helpers to shared, typed Rust implementations.
- Prefer streaming/iterator-based outputs over large JSON blobs to meet performance targets.
