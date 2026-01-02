# Aideon Suite - Staged Implementation Roadmap

## Purpose

Outline the forward-looking milestones and phases for Aideon Suite, showing how Praxis, Chrona,
Metis, Continuum, and Mneme evolve over time. This document focuses on **what** we plan to deliver
and in which stage, not on low-level architecture or design decisions.

**Date:** 2025-10-14
**Goal:** Deliver **Aideon Suite**, a local-first, graph-native EA platform (Tauri desktop + Rust
engines) that treats **time and scenarios as first-class dimensions** (valid time + asserted time
+ Plan/Actual layers), with a clean path to server mode. The current implementation focus is the
**Praxis desktop module** (core digital twin), with Chrona, Metis, Continuum, and Mneme evolving
alongside it.

## Guiding Principles

- Clear separation: Tauri/Rust host & OS integration; engine crates (Praxis/Chrona/Metis/Continuum)
  behind typed traits.
- Swap-friendly adapters: Graph, Storage, RPC are interfaces with reference impls.
- Security by default: hardened IPC, PII redaction, least privilege.
- Cloud-ready: Same components run local or remote by configuration.
- Small, valuable increments: every phase has acceptance and exit checks.

## Milestones

### M0 - Foundations (Weeks 1-2)

#### Outcomes

- Monorepo, CI for TS and Rust, CODEOWNERS.
- Tauri capabilities baseline (no renderer HTTP, strict CSP, least-privilege plugins).
- Architecture boundaries documented and enforced.
- Interfaces: `GraphAdapter`, `StorageAdapter`, `WorkerClient`.

> **Renderer migration:** The legacy Svelte renderer has been removed. All roadmap items assume
> the React + React Flow canvas runtime described in `docs/DESIGN.MD` and
> `app/AideonDesktop/docs/praxis-workspace/DESIGN.MD`. Any remaining Svelte references (primarily in
> mirrored historical issue docs) are informational only.

#### Acceptance

- `pnpm run node:test` green on macOS/Windows/Linux; Rust host `cargo fmt/clippy/check/test` clean.
- App shell launches with secured preload/IPC bridge; no renderer HTTP.
- Architecture docs updated and agreed (RPC and adapter boundaries).

### M1 - Local App MVP (Weeks 3-6)

#### Outcomes (updated)

- React + Tauri desktop shell (typed IPC) backed by local Mneme store.
- Praxis metamodel packages + registry bootstrapped with stable IDs.
- Time cursor (scenario + valid time + layer) drives time-sliced reads.
- Basic artefact execution in the UI (views/catalogues) with empty/loading/error states.
- Pipeline hardening (coverage in CI, CSP checks, Sonar inputs).

#### Acceptance (updated)

- User can switch scenario, adjust time, and see artefact results update.
- CI: Node + Rust tests green; coverage >= 80% new code; CSP/windows test passes.

### M2 - Analytics Worker MVP (Weeks 7-10)

#### Outcomes

- Worker process for analytics (Metis) behind RPC; per-launch token.
- Algorithms: shortest path, degree/betweenness, impact analysis.
- Arrow for large payloads; health and restart supervision.
- Perf baselines captured (SLOs).

#### Acceptance

- Worker jobs return deterministically; no open TCP ports.
- 50k-node centrality within target SLOs.

### M3 - Artefacts + Time APIs (Weeks 11-14)

#### Outcomes

- Artefact schemas and storage (views, catalogues, matrices, maps).
- Plan Events model (+ confidence) and time/scenario execution context.
- AS-OF slider, scenario compare export, catalogue/matrix drill-down.
- CSV wizard v2: validation/rollback.

#### Acceptance

- Time-slicing works end-to-end; artefact execution stable and explainable.
- BI reads via localhost; redaction verified.

### M4 - Automation and Connector #1 (Weeks 15-18)

#### Outcomes

- Scheduler; CMDB connector (delta sync).
- Staleness metrics; freshness story mode.

#### Acceptance

- Scheduled syncs update changed items only; audit logs kept.
- Staleness dashboard active.

### M5 - Cloud/Server Mode (Weeks 19-24)

#### Outcomes

- Remote Graph/Worker endpoints (mTLS), config switch local<->remote.
- RBAC + optimistic locking + audit.
- Finance/TCO policies (inflation, FX, discount) and `GET /tco`.
- Landing Zone catalogue + conformance.

#### Acceptance

- Desktop targets local or remote without code changes.
- Concurrent edits resolve with clear UX.
- TCO scenario PV compare report generated.

### M6 - Docs and Extensibility (Weeks 25-28)

#### Outcomes

- Full C4 suite (DSL + exports), docs site build.
- Plugin hooks and examples.

#### Acceptance

- One-command docs site build; sample plugins pass tests.

## SLOs (v1 Targets)

- Cold start <= 3s; open workspace (~50k/200k) <= 2s.
- View execution p95 <= 250ms at 50k/200k (warm).
- Matrix execution (1k x 1k sparse) <= 1s; export <= 2s (500 items).
- Shortest path (<= 6 hops) p95 <= 300ms; betweenness (50k) <= 90s batch.
- CSV import 50k rows <= 120s; UI p95 render <= 100ms.

## Non-Goals (MVP)

- Full OWL/SHACL reasoning, marketplace plugins, AR/VR, multi-tenant SaaS.

## Immediate Backlog (Prioritised)

1. Prove the pipeline and harden

- Persistence boundary: keep `continuum::SnapshotStore` for canvas/layout data; use Mneme facts for
  domain state.
- E2E contract tests: Vitest against mocked Tauri bridge with golden JSON snapshots for artefact
  execution and time-context changes.
- Coverage in CI: generate LCOV (Vitest) and Rust coverage (grcov); feed Sonar (ensure report
  paths).
- Security checks: assert CSP and window permission caps in a prod build test.
- Release dry run: produce dev artifacts for macOS/Windows/Linux; verify signing path (ad-hoc certs
  locally acceptable).

2. Time & Scenarios (M1.1)

- Timebar (playhead/scrubber), scenario picker, layer toggle, filters, compare toggle, status area
  for unsaved changes.
  Acceptance: scrub time, switch scenarios, enter compare in <= 2 clicks.

3. Graph Edit & Inspect (M1.2)

- Canvas affordances (pan/zoom, snap-to-grid, lasso, drag handles to create edges), context actions,
  inspector panel, keyboard shortcuts.
  Acceptance: create two nodes, connect, edit labels, undo/redo in <= 30s.

4. Change Authoring & Diff (M1.3)

- Task-oriented authoring (create/link/update), diff view, conflict tray, preview.
  Acceptance: create a scenario change, compare to baseline, resolve a conflict.
