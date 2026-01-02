# Aideon Host – Internal Design

## Overview

Aideon Host is the Tauri-based entrypoint for the desktop app. It owns window lifecycle, OS
integration, capabilities, and the typed IPC surface for calling engine crates from the React
renderer.

## Internal structure

- Tauri app/bootstrap code and window creation.
- Command registration and routing to engine traits.
- Capability configuration and CSP (see `tauri.conf.json` and capabilities files).
- Logging and basic health checks exposed to the renderer.

## Data model and APIs

- Defines command DTOs and error types that cross the host boundary.
- Maps IPC commands to engine/facade methods (Praxis Engine, Chrona, Metis, Continuum, Mneme).

## Interactions

- Renderer ↔ Host: typed IPC commands, no HTTP.
- Host ↔ Engines: in-process trait calls in desktop mode; future remote adapters must preserve the
  same contracts.

## Constraints and invariants

- No open TCP ports in desktop mode.
- Renderer cannot bypass the typed command surface.
- PII redaction and security posture follow `docs/TAURI-CAPABILITIES.MD` and
  `docs/TAURI-CLIENT-SERVER-PIVOT.MD`.

## Patterns in use

- Errors: map engine `PraxisError` to lightweight `HostError` codes (`temporal.rs` is the golden path; new commands should mirror it).
- Logging/tracing: `log` macros with clear prefixes (`host:`) and timing where relevant; no custom loggers.
- Async/runtime: `tokio` async, `tauri::State` for managed engines; avoid bespoke threading.
- IO: use Tauri-provided paths (`app_data_dir`) and `mneme` datastore helpers; no ad-hoc file handles.
- Examples: `temporal.rs` (state/diff/topology) and `health.rs` now serve as paired examples of the host command style.

## Implementation notes

- Errors: propagate engine `PraxisError` into lightweight host codes; keep mapping in one place
  (`HostError`).
- Logging: standard `log`/`tracing` macros; avoid custom loggers.
- Async/concurrency: use `tokio` primitives; no bespoke threading.
- Paths: obtain app data dir via `tauri::path`; avoid hardcoded filesystem paths.
