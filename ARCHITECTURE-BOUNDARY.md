# Architecture and Boundaries (Aideon Suite)

## Purpose

This document defines the **boundary rules** that keep Aideon modular, secure, and evolvable. It is
the shared reference for “what can talk to what” and “what must never happen”.

---

## Documentation precedence

Documentation is authoritative. Update code to match documentation. Do not change documentation to match existing code unless the intended architecture changes.

---

## The boundary thesis

- **Renderer is disposable UI.** It should be safe to restart, replace, or refactor without losing
  model correctness.
- **Host is the authority for side effects.** All IPC, capabilities, filesystem access, and job
  orchestration are centralized.
- **Engines are replaceable.** They expose typed trait contracts and can be swapped (local vs
  remote adapters) without changing the renderer.

---

## Layers and responsibilities

### Renderer (React)

Allowed:

- render artefact results and diagram specs
- maintain UI state (selection, filters, tabs, local layout)
- call host via typed adapters

Forbidden:

- Node integration or OS access
- direct DB access
- raw HTTP requests (desktop baseline)

### Host (Tauri)

Allowed:

- typed IPC surface (commands + events)
- capability enforcement and CSP
- workspace lifecycle (open/close/migrate/backup)
- job orchestration (progress, cancel, recovery)
- OS integration (dialogs, filesystem roots, windowing)

Forbidden:

- embedding domain semantics in IPC handlers
- letting long-running work block an IPC request/response

### Engines (Rust crates)

Allowed:

- domain logic behind traits (Praxis, Mneme, Metis, Chrona, Continuum)
- deterministic computation independent of UI/runtime
- persistence only through Mneme (no raw DB access outside Mneme)

Forbidden:

- importing Tauri/UI dependencies into engine crates
- engines emitting UI events directly (host owns event publication)

---

## Communication paths (allowed)

Renderer ↔ Host uses Tauri invoke. Host ↔ Engines uses Rust traits in-process.

### Renderer -> Host

- Typed IPC only (commands for request/response, events for push).
- Renderer code must call host through adapter modules under `app/AideonDesktop/src/adapters`.

### Host -> Engines

- In-process trait calls in desktop mode.
- Remote adapters are allowed in server mode, but must preserve the same DTO contracts.

---

## IPC contract rules

- Command names are namespaced: `<domain>.<capability>.<action>`.
- Payloads are single JSON objects (no positional args).
- Responses use a stable error envelope with machine-readable codes.
- Commands are capability-gated; default is deny.

### Tauri capability sources (desktop baseline)

- Capability definitions: `crates/desktop/capabilities/default.json`
- Tauri config: `crates/desktop/tauri.conf.json`
- Command implementations: `crates/desktop/src`

---

## Time-first propagation rule (suite-wide)

All read/write operations must carry explicit context:

- `partition_id`
- time context: valid time, asserted time (optional), layer
- `scenario_id` (optional)

No module is allowed to assume “current state only”.

---

## Artefact execution boundary

- Praxis executes artefacts and returns UI-ready results and diagram specs.
- Renderer renders results and handles interaction; it does not implement traversal semantics.
- Bounded execution is mandatory (depth/size/fanout/time limits).

---

## Jobs, progress, and backpressure

- Long-running work runs as jobs, not synchronous IPC handlers.
- Host exposes job lifecycle and progress events to the renderer.
- Engines register work with host job orchestration; they must not spawn unmanaged background work
  that cannot be cancelled or observed.

---

## Security constraints (desktop baseline)

- No renderer HTTP.
- No open TCP ports in desktop mode.
- Filesystem access is mediated by host and scoped to workspaces/app data.
- Exports are deny-by-default for PII; redaction is required where exports exist.

---

## Versioning and evolution

- DTO and error envelope changes require updating contract docs and contract tests.
- Schema evolution is forward-only and migrated explicitly.
- Server pivot is an adapter swap, not a UI fork.

### Pivot constraints (desktop -> remote)

- Renderer continues to call typed adapters only.
- Host selects local vs remote implementations behind the same DTOs and command names.
- Baseline security invariants remain true (no renderer HTTP; host remains the boundary).

---

## References

- Suite design: `docs/DESIGN.md`
- Host design: `crates/desktop/DESIGN.md`
- Praxis design: `crates/praxis/DESIGN.md`
- Mneme design: `crates/mneme/DESIGN.md`
- Contracts: `docs/CONTRACTS-AND-SCHEMAS.md`
