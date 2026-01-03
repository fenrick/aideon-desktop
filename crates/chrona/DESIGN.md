# Chrona Visualisation – Design

## Purpose & scope

Chrona Visualisation produces time-aware views (state-at, diff, topology delta) over the Praxis
digital twin. It shapes data for timelines, time sliders, and visual overlays consumed by Praxis
Canvas and other clients.

## Allowed dependencies / frameworks

- Rust 2024 with workspace defaults.
- Core deps: `tokio` for async, `serde`/`serde_json` for (de)serialisation, `thiserror` for errors,
  `tracing` + `log` facade for logging.
- Twin access via `praxis` and persistence through `mneme` traits only;
  no direct DB drivers here.

## Anti-goals

- No renderer/UI, Tauri, or HTTP servers.
- No direct database access; persistence must flow through Mneme traits.
- No bespoke async runtimes or logging frameworks.

## Public surface

- Traits/functions that expose temporal summaries: `state_at`, `diff`, `topology_delta`, and
  timeline-friendly aggregates.
- Data transfer structs tuned for canvas widgets (timeline segments, plateau/gap markers,
  selection overlays).

## Evergreen notes

- Migrate any legacy, untyped JSON helpers to typed structs + serde.
- Replace ad-hoc diff logic with shared semantics defined in Praxis engine design docs.
