# Aideon Suite - Evergreen Design Overview

## Purpose

This is the **central, cross-module design spec** for Aideon Suite. It captures the common threads
that must stay true across renderer, host, and engine crates.

Module details live in:

- Host: `crates/desktop/DESIGN.md`
- Praxis: `crates/praxis/DESIGN.md`
- Mneme: `crates/mneme/DESIGN.md`

---

## Design axioms (non-negotiable)

1. **Time-first digital twin**
   - All reads/writes operate in an explicit time context:
     - valid time (modeled reality)
     - asserted time (what we knew, when)
     - layer (Plan vs Actual precedence)
     - optional scenario (what-if overlay)

2. **Ops are the canonical truth**
   - An append-only op log is the durable source of record.
   - Derived tables (indexes, projections, caches, analytics outputs) are rebuildable.

3. **Meaning is separate from storage**
   - Mneme persists facts and schema-as-data.
   - Praxis defines semantics (metamodel), tasks, artefacts, integrity, and explanations.

4. **Artefacts are the primary UX product**
   - Users consume views/catalogues/matrices/maps/reports/pages executed at time + scenario.
   - UI does not embed traversal rules or analytics meaning.

5. **Host is the security boundary**
   - Renderer is untrusted and disposable; side effects flow through host capabilities.
   - Desktop mode is offline-first: no renderer HTTP and no open TCP ports.

6. **Bounded and explainable execution**
   - All user-triggered work has explicit bounds (fanout, size, depth, duration).
   - Analytics and time-travel outputs must be explainable, not just computed.

---

## The combined system model

Aideon Suite is delivered as a secure desktop app with a clean pivot to client-server mode.

- **Renderer (React)** renders artefact results and UI state only.
- **Host (Tauri/Rust)** owns IPC, capabilities, jobs, workspace lifecycle, and OS integration.
- **Engines (Rust crates)** implement meaning, persistence, analytics, and orchestration behind
  typed traits.

The pivot to server mode is an adapter swap: the renderer keeps the same typed command surface
while the host selects local vs remote implementations.

---

## Shared concepts (suite vocabulary)

### Workspace / partition

A workspace (partition) is the unit of isolation and user work:

- all facts and schema are partition-scoped
- scenarios are overlays within a partition
- artefacts are stored and executed per partition

### Time context

Time is not ambient:

- valid time answers “what is true?”
- asserted time answers “what did we know?”
- layer answers “plan or actual wins?”
- scenario answers “baseline or overlay?”

### Strategy-to-execution spine (normative)

Praxis centers the twin around a stable backbone used for exploration, integrity checks, and
analytics gating:

**Intent -> Value -> Capability -> Execution -> Technology -> Change**

This spine is not a UI flow; it is a semantic expectation that drives:

- integrity scoring (connectivity gaps along the spine reduce confidence)
- bounded traversal defaults for artefact execution
- explainability (“why does this capability matter?” is expressed as paths along the spine)

### Glossary (shared nouns)

- **Partition**: the workspace/tenant boundary for all data and schema.
- **Scenario**: an overlay within a partition for what-if changes.
- **Master type**: a stable structural role (Actor, Intent, Value, Capability, Execution, Technology, Structure, Change).
- **Domain type**: an extensible business concept inheriting from a master type.
- **Element**: a node instance of a domain type.
- **Relationship**: an edge instance of a domain verb mapped to master semantics.
- **Task**: a user-facing authoring operation (create/link/set/move/tombstone).
- **Artefact**: a stored definition executed to produce results (view, catalogue, matrix, map, report/page).

### Tasks and artefacts

Praxis exposes:

- **Tasks**: authoring operations that mutate the twin (create/link/set/move/tombstone, etc.).
- **Artefacts**: declarative work products executed at a time/scenario to produce UI-ready results.

### Jobs and events

Anything long-running or failure-prone is a job:

- migrations, imports/exports, analytics, background processing
- cancellable, observable, and recoverable where feasible
- progress/events emitted by host to keep UI responsive

---

## End-to-end flow (facts to UX)

1. User triggers a **task** (Praxis validates semantics).
2. Praxis writes through **Mneme** (ops + facts; required projections/indexes maintained).
3. Praxis executes **artefacts** at an explicit time context and returns:
   - result data
   - diagram specs and hints
   - integrity gates/warnings and explainability
4. Renderer renders results and drives inspector/actions from selection.

---

## Security posture (desktop baseline)

- No renderer HTTP; renderer calls host via typed IPC only.
- No open TCP ports in desktop mode.
- Filesystem access is workspace-scoped and mediated by host dialogs + capabilities.
- Exports are deny-by-default for PII; redaction is enforced where applicable.

---

## Evolution and compatibility

- DTOs and error envelopes are versioned; forward compatibility is required.
- Schema evolution is forward-only with explicit migrations.
- Cross-boundary changes must update code + tests + contract docs together.

---

## References (authoritative entry points)

- Boundaries: `ARCHITECTURE-BOUNDARY.md`
- UX contract: `docs/UX-DESIGN.md`
- Desktop shell: `app/AideonDesktop/DESIGN.md`
- Contracts: `docs/CONTRACTS-AND-SCHEMAS.md`
