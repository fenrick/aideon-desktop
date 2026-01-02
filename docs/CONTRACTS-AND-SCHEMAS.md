# Contracts and Schemas

## Principle

- **Contracts first, implementation second.** Rust, Tauri host, and React renderer share a single contract per boundary. Implementations map to that contract; ad-hoc DTOs are not allowed.

## Where contracts live

- **TypeScript (renderer/adapters):** `app/AideonDesktop/src/dtos` is the canonical source for renderer DTOs. Feature code imports types from this package only; do not redefine payload shapes in feature modules.
- **Rust (host/worker):** `crates/mneme` owns the shared IPC/engine DTOs (e.g., temporal, health). Host commands and worker crates use these structs directly.

## Synchronisation model

- **Manual mirroring with tests.** We do not yet generate code from a schema. Rust DTOs (serde) are treated as canonical for host/worker IPC; TypeScript mirrors live in `app/AideonDesktop/src/dtos` with matching field names and casing. Contract tests in both stacks guard drift.
- **Error shapes:** Host errors are normalised in `crates/desktop::temporal::host_error` and must be reflected in renderer-facing docs/tests whenever they change.

## How to change a contract

1. Update the canonical DTO (Rust struct in `crates/mneme`, mirrored type in `app/AideonDesktop/src/dtos`).
2. Adjust host command handlers and renderer adapters to consume the shared type instead of redefining it.
3. Extend contract tests (Rust + TypeScript) to cover the new/changed shape.
4. Note the change here and in any relevant module `README.md`/`DESIGN.md` sections.
