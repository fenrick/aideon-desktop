# Rectification Plan

**Scope:** Close architectural gaps between the current codebase and the evergreen Praxis/Mneme
specs. This plan is aligned to `crates/praxis/DESIGN.md` and `crates/mneme/DESIGN.md` and focuses on
bringing storage, metamodel, artefacts, and analytics into a cohesive, time-first digital twin.

## Core objectives

1. **Mneme-first storage**: Move all persistence, schema, facts, and analytics projections behind
   Mneme APIs (no direct SQL, no renderer DB access).
2. **Task-oriented Praxis**: Implement task APIs, metamodel packages, and domain registry with stable
   IDs.
3. **Artefact execution**: Views, catalogues, matrices, maps, and reports are stored and executed
   with explicit schemas.
4. **Integrity + explainability**: Enforce directionality, logical/physical separation, and spine
   completeness; surface integrity scores and explainability paths.
5. **Desktop-safe boundaries**: Renderer <-> Host <-> Engines over typed IPC only; no open ports in
   desktop mode.

## Workstreams

| ID   | Workstream                                | Primary Modules                    | Depends on |
| ---- | ----------------------------------------- | ---------------------------------- | ---------- |
| WS-A | Mneme storage + facts + projections        | Mneme, Praxis                      | None       |
| WS-B | Metamodel packages + registry + rules      | Praxis, Mneme                      | WS-A       |
| WS-C | Artefact schemas + execution + UI adapters | Praxis, Host, Desktop              | WS-A/B     |
| WS-D | Analytics + explainability                | Metis, Praxis, Host                | WS-A/B/C   |
| WS-E | Governance, CI, and coverage gates         | Docs, CI, Host, Desktop, Engines   | WS-A-D     |

## WS-A - Mneme storage + facts + projections

**Objective:** All persistence is handled by Mneme using ops + facts + schema tables and projection
edges. No engine or UI generates SQL.

- Mneme SeaORM migration coverage matches the authoritative schema in `crates/mneme/DESIGN.md`.
- Host initializes Mneme store from application data paths (no repo-relative storage).
- Projection edges maintained transactionally for analytics.

**Definition of Done**

- Store opens, migrates, and seeds successfully on all platforms.
- Read/write APIs function with valid-time + asserted-time semantics.
- Projection edges update on edge create/tombstone without background workers.

## WS-B - Metamodel packages + registry + rules

**Objective:** Praxis publishes metamodel packages with stable IDs, builds a domain registry, and
enforces master semantics + rule gates.

- Package compiler outputs Mneme `MetamodelBatch` (types/fields/type_fields/edge_rules).
- Registry maps domain keys and verbs to Mneme IDs.
- Rule engine validates directionality and logical/physical separation.

**Definition of Done**

- Baseline packages install cleanly into Mneme.
- Task APIs validate against registry + rules.
- Integrity scans produce structured findings and scores.

## WS-C - Artefact schemas + execution + UI adapters

**Objective:** Artefacts (views/catalogues/matrices/maps/reports) are stored as data with explicit
schemas and executed in Praxis with time/scenario context.

- JSON schemas defined and versioned per artefact kind.
- Execution pipelines return UI-ready results (nodes/edges/groups/summary).
- Renderer uses typed IPC to run artefacts and show loading/error/empty states.

**Definition of Done**

- At least 1 view, 1 catalogue, 1 matrix, and 1 map run end-to-end in desktop mode.
- Artefact validation blocks unsafe traversal or unbounded queries.
- Drill-down APIs return element and relationship summaries.

## WS-D - Analytics + explainability

**Objective:** Analytics runs against Mneme projection edges and is explainable in domain terms.

- Metis implements centrality/impact/TCO jobs with deterministic outputs.
- Praxis orchestrates analytics and stores results with integrity context.
- Explainability returns top paths and contributing dependencies.

**Definition of Done**

- Analytics runs from UI, completes within target SLOs, and returns explainable results.
- Results are cached with invalidation on data change.

## WS-E - Governance, CI, and coverage

**Objective:** CI enforces quality gates, schema/artefact validation, and documentation alignment.

- CI runs dataset and schema validation.
- Coverage gates for TS and Rust maintained or improved.
- Design docs updated for any boundary or protocol changes.

**Definition of Done**

- `pnpm run ci` passes on macOS/Windows/Linux.
- Coverage >= 80% new code for TS and >= 90% for core engine logic.
- Documentation matches module design specs.

## Test requirements (cross-cutting)

- Mneme: unit tests for facts resolution and projection updates.
- Praxis: tests for metamodel compilation, rule violations, artefact validation.
- Host: IPC payload tests for time/scenario parameters and error shapes.
- Desktop: UI tests for loading/error states and drill-down workflows.

## Tracking

Each workstream should map to GitHub issues with DoD checklists and links to the updated design docs.
