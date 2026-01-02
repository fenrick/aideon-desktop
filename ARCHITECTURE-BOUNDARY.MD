# Architecture & Boundaries (Aideon Suite)

## Purpose

Provide the canonical code-level architecture and boundary reference for the Aideon Suite monorepo:
how renderer, host, and engine crates are layered; how adapters and RPC work; and which security and
time-first constraints must hold across modules (Praxis, Chrona, Metis, Continuum, Mneme).

## Layers

- **Renderer**
  - React-based Praxis workspace runs inside the Tauri shell.
  - Renders artefacts (views, catalogues, matrices, maps) and inspectors; owns view state only
    (selection, filters, time context).
  - No Node integration; strict CSP; no direct DB or network access; all backend calls go through a
    typed IPC bridge.

- **Host (Tauri)**
  - Creates windows and binds typed commands that wrap engine traits.
  - Owns capabilities, CSP, window configuration, logging, and OS integration.
  - Enforces "no open TCP ports" in desktop mode; all work is in-process or over local IPC.

- **Adapters (TypeScript)**
  - Renderer-facing contracts for **tasks**, **artefact execution**, and **analytics**.
  - Implementations remain backend-agnostic; renderer code depends on interfaces only.

- **Engines (Rust crates)**
  - Praxis, Mneme, Chrona, Metis, Continuum expose computation traits consumed by the host.
  - Desktop mode uses in-process adapters; server mode swaps in remote adapters that honor the same
    trait contracts.

- **Persistence & schema (Mneme + Praxis)**
  - Mneme Core provides the ACID store (SQLite in desktop mode) and schema/fact tables.
  - Praxis publishes metamodel packages into Mneme and enforces semantic rules.

## Host <-> Engine RPC boundary

Desktop mode keeps all work **in-process** behind Rust traits, but the same contracts are designed
for remote/server adapters over pipes/UDS or HTTP/2 when enabled.

### Transport, auth, and schema

- **Transport (desktop):** in-process calls over trait objects; no open TCP ports.
- **Transport (server mode, future):** pipes/UDS or HTTP/2 with the same DTOs.
- **Auth:** per-launch capability token, per-job deadlines, backpressure, and timeouts.
- **Schema:** DTOs are versioned and compatible across local and remote adapters.

### Core messages

- **Job**: `id`, `type`, `schemaVersion`, `payloadRef` (JSON or Arrow), `options`, `deadline`.
- **Result**: `id`, `status`, `payloadRef` (JSON or Arrow), `metrics` (duration, bytes), optional `error`.
- **Health**: `status`, `versions`, `uptime`.

### Job types (v1)

- `Analytics.Centrality` - `{ algorithm: degree|betweenness, scope }`
- `Analytics.Impact` - `{ seedRefs[], filters{} }`
- `Finance.TCO` - `{ scope, asOf, scenario?, policies? }`
- `Temporal.StateAt` - `{ asOf, scenario?, layer? }` (time-context read)
- `Temporal.Diff` - `{ from, to, scope? }`

### Data transfer and lifecycle

- **Small payloads:** JSON (UTF-8).
- **Large/columnar payloads:** Apache Arrow; optional Arrow Flight over gRPC in server mode.
- Payloads are immutable; results include provenance (time context, policy set IDs).

## Boundaries & Security

- Renderer has no direct Node or backend access.
- Host <-> Renderer: preload IPC only; strict CSP applied in HTML.
- Host <-> Worker: in-process Rust traits today; future remote adapters must preserve the same
  command surface. No open ports in desktop mode.
- PII: exports are deny-by-default; redaction tests required where applicable.

## Time-first model (facts + context)

- **Valid time**: when something is true in the modeled world.
- **Asserted time**: when the system learned the fact.
- **Layer**: Plan vs Actual (higher layer wins in conflicts).
- **Scenario**: overlay context for what-if changes.

All reads and writes must include explicit time context. Mneme resolves facts deterministically
based on valid time, layer precedence, interval specificity, and asserted time.

## Metamodel enforcement

- Praxis publishes metamodel packages into Mneme schema tables (types, fields, edge rules).
- Domain registry maps domain keys and verbs to Mneme IDs.
- Writes are validated against master semantics and endpoint constraints.

## Artefact execution boundary

- Artefacts (views, catalogues, matrices, maps, reports) are executed by Praxis with explicit time
  context and scenario.
- Renderer never builds traversal logic; it consumes artefact results and diagram specs only.

## Performance guardrails

- Artefact execution and analytics must respect SLOs in `docs/ROADMAP.MD`.
- Large payloads must be chunked/streamed; avoid monolithic JSON blobs.

## Versioning & migration

- Breaking DTO or storage changes require explicit version bumps and migrations.
- Schema evolution is forward-only; migrations are explicit and logged.

## Compliance checklist

- [x] No renderer HTTP
- [x] No open TCP ports in desktop mode
- [x] Renderer invokes host commands through typed helper modules (no preload globals)
- [x] Worker logic executes via engine traits
- [x] Versioned DTOs and schema migrations
