# Roadmap (Evergreen Planning)

## Purpose

Capture the staged delivery plan for Aideon Suite. This document is **planning**, not architecture.
Design truth lives in `docs/DESIGN.md` and `ARCHITECTURE-BOUNDARY.md`.

Last reviewed: 2026-01-02

---

## Planning rules

- Desktop-first and offline-first by default.
- No renderer HTTP; no open TCP ports in desktop mode.
- Renderer remains stable: locality (local vs remote) is an adapter swap behind the host.
- Work ships as bounded, explainable artefacts and tasks; no unbounded “query UI”.
- Backlog lives in GitHub Issues/Projects; this doc only records milestones and SLO targets.

---

## Milestones

### M0 - Foundations

- Secure desktop baseline (typed IPC, CSP, capabilities).
- Contract discipline (DTOs + error envelopes + contract tests).
- Mneme store available in desktop mode (SQLite) with migrations.

### M1 - Local app MVP (Praxis workspace)

- Praxis metamodel bootstrapped with stable IDs.
- Time/scenario context drives artefact execution and UI controls.
- Initial artefacts rendered in the UI with loading/empty/error states.

### M2 - Analytics execution (Metis)

- Analytics jobs wired as host-managed work (job lifecycle + progress).
- Deterministic algorithms and performance baselines.
- Large payload strategy (streaming/columnar where needed).

### M3 - Artefact system depth

- Artefact storage and schema validation (views/catalogues/matrices/maps/reports/pages).
- Drill-down and explainability surfaced consistently across artefacts.
- Scenario compare and time-diff UX grounded in artefact outputs.

### M4 - Automation (Continuum)

- Scheduling + connector scaffolding behind adapters.
- Ingest workflows that preserve provenance (ops + facts) and stay replayable.

### M5 - Server mode pivot

- Remote adapters for engines while preserving renderer contracts.
- Authn/z, audit, and capability posture consistent with desktop baseline.

### M6 - Extensibility + docs hardening

- C4 diagrams-as-code and contract documentation tightened.
- Plugin/extension seams proven by at least one non-core module.

---

## SLO targets (v1)

- Cold start <= 3s
- Open medium workspace (tens of thousands of entities) <= 2s
- Interactive artefact execution p95 <= 250ms (warm)
- Matrix execution (1k x 1k sparse) <= 1s; export <= 2s (500 items)

---

## Non-goals (v1)

- Full OWL/SHACL reasoning.
- Marketplace plugins and multi-tenant SaaS productization.
