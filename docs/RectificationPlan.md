# Aideon Suite — Rectification Plan (Debt & Gap Analysis)

## Purpose

Capture architectural and implementation debt for Aideon Suite (storage, meta-model, dataset,
analytics, and governance gaps) and organise it into workstreams that can feed the roadmap and ADRs.
This document is used for internal gap analysis; roadmap and decision records live in `docs/ROADMAP.md`
and `docs/adr/` respectively.

**Author:** Codex Agent  
**Date:** 2025-11-10  
**Scope:** Close architectural gaps (git-structured storage, abstract meta-model, configurable schema delivery, baseline dataset, reporting/analytics) identified during audit.

## 0. Identified Gaps in Design

### Git-structured timeline

The intent in Architecture-Boundary.md:75–146 is a Git-like, append-only commit graph with deterministic ancestry and timeline semantics, yet PraxisEngine just keeps commits/branches in BTreeMaps behind a mutex and increments next_commit for ids (crates/praxis/src/engine.rs:37 and crates/praxis/src/engine.rs:159), meaning history evaporates on restart and commit timestamps are optional/unused; even the UI fell back to showing the id when time was missing (legacy Svelte renderer, now removed). Wire PraxisEngine to a persistent commit/tag store that records authoritative snapshot markers inside SQLite, stamp commit metadata server-side, and surface branch pointers so “main” always reflects “now” without trusting the renderer.

### Abstract meta-model enforcement

The design requires every node/edge to obey the ArchiMate-style catalog (docs/DESIGN.md:83–196), but the engine only stores opaque NodeVersion/EdgeVersion records with TODOs where schema validation should live (crates/praxis/src/engine.rs:183 and crates/praxis/src/graph.rs:235); StateAtResult today exposes counts only (crates/mneme/src/temporal.rs:24), so no meta-model attributes reach consumers. Introduce a MetaModelRegistry that materialises the schema definitions from commit-style data (e.g., `docs/data/meta/core-v1.json` seeded during import) so the same nested graph structure describing object types, attributes, and constraints is versioned alongside regular commits, enforced inside `GraphSnapshot::apply`, and surfaced as typed DTOs the renderer consumes without embedding business logic.

### Configurable/deliverable meta-model

The delivered meta-model is only prose in docs/DESIGN.md, and there’s no mechanism to configure it or ship extensions; even the analytics contract (app/AideonDesktop/src/adapters/src/contracts.ts:145) lists job types but nothing populates them. Instead of keeping schema definitions in static files, treat meta-model configuration as data that can be authoring via in-app screens—users compose object types in a graph-style hierarchy, define attributes and constraints, and commit the resulting descriptor; the baseline offering (`docs/data/meta/core-v1.json`) is just the default data payload. This data-first approach lets overrides land as additional commits or files under `.praxis/meta`, and the host exposes the active schema so UI/business logic can materialise different viewpoints without code changes.

### Base dataset alignment

Requirements call for a baseline dataset that mirrors the strategy-to-execution chain, yet the engine seeds just five sample nodes and four edges (crates/praxis/src/engine.rs:425) and the renderer’s dev adapter merely tracks node/edge counts in memory (app/AideonDesktop/src/adapters/src/development-memory.ts:14). Build an importer that loads the real delivered dataset (value streams, capabilities, applications, plan events, etc.) into commits, version it alongside migrations, and ensure state_at can stream actual graph data (not just counts) to both the canvas and reporting layers.

### Reporting & analytics surface

The design promises executive dashboards, scorecards, and roadmap visuals (docs/DESIGN.md:860–884), but today the renderer shows only snapshot counts and diff totals in MainView (legacy Svelte renderer, now removed), the canvas pulls a hard-coded two-rectangle scene (crates/chrona/src/scene.rs:5), StateAtResult lacks any graph detail, and Metis is an empty placeholder (crates/metis/src/lib.rs:1). Deliver the analytics worker jobs defined in the contracts file (app/AideonDesktop/src/adapters/contracts.ts:145), add host commands to invoke them, persist/report their outputs (heatmaps, TCO, timeline charts), and expand.

## 1. Guiding Principles

- **Time-first, git-native:** Commits are the single source of truth; wall-clock timestamps are metadata only.
- **Local-first, cloud-ready:** Desktop persists data locally but uses interfaces that can swap to remote backends.
- **Strict boundaries:** Renderer stays IPC-only; all domain logic lives in host/worker crates.
- **Security by default:** No renderer HTTP, no new ports, PII redaction enforced in reporting.
- **Incremental delivery:** Each workstream lands behind tested commits, documented in GitHub issues with DoD checklists.

## 2. Workstream Overview

| Workstream | Goal                                          | Primary Owners            | Dependencies              |
| ---------- | --------------------------------------------- | ------------------------- | ------------------------- |
| WS-A       | Git-structured temporal storage & persistence | Praxis / Continuum        | None (foundation)         |
| WS-B       | Meta-model enforcement & configurability      | Praxis / Core Data        | WS-A (storage primitives) |
| WS-C       | Delivered baseline dataset & tooling          | Praxis / Docs             | WS-A/B (schema + storage) |
| WS-D       | Analytics & reporting functionality           | Metis / Chrona / Renderer | WS-B/C (schema + data)    |
| WS-E       | Process/quality guardrails & tracking         | Repo-wide                 | Parallel (supports all)   |

Each workstream below details steps, Definition of Done (DoD), testing, and commit guidance.

## WS-A — Git-Structured Temporal Storage

**Objective:** Replace the in-memory temporal store with an async, SeaORM-backed persistence layer that keeps commits/refs/snapshots in SQLite, alongside a readonly Metis-friendly fact/star schema, and seeds the baseline dataset (including the meta-model) at setup.

### Status

**Partially complete**

- `SqliteDb` now persists commits/refs/tags via SeaORM and `PraxisEngine::ensure_seeded` replays the embedded baseline dataset whenever `main` is empty so fresh installs have realistic history.
- `create_datastore`, `cargo aideon_xtask import-dataset`, and `cargo aideon_xtask health` provide the bootstrapping and verification tooling needed to stand up and audit SQLite datastores.

**Outstanding**

- Build analytics rollups (per-branch timelines, contributor/plan-event aggregates) and extend regression coverage for branch merge/diff scenarios so future migrations stay fully data-driven now that commit/node/edge facts are captured.

### Tasks

1. **Async trait conversion**
   - Update `mneme::Store` to an async trait (via `async-trait`) so SeaORM can power commit, branch, and tag operations without blocking the host runtime.
   - Modernise every caller (`PraxisEngine`, host worker, aideon_xtask CLI, tests) to await the store operations without changing the visible API surface.
   - _Completed — the store trait is async end-to-end, `SqliteDb` executes directly on the caller runtime, and all engines/hosts/tests await the new futures._
2. **SeaORM schema & migrations**
   - Introduce SeaORM 1.1.19 + SeaQuery in `crates/mneme`, define entities for commits, refs, snapshot tags, and the readonly Metis fact/star tables.
   - Build migrations that create the tables with the required constraints and indexes, and run them at startup via a SeaORM migrator.
   - _In progress — `mneme_migrations` now records applied versions; commit/node/edge fact tables populate during projections, but aggregated rollups and merge regression coverage remain._
3. **Baseline dataset seeding**
   - Seed the baseline dataset (meta-model, object graph, plan events) as part of the migration/seed logic so a fresh database already contains the canonical schema and sample graph.
   - Record the same event stream into the Metis-ready tables so the analytics worker can later query a flattened, star/fact-oriented view of the commits.
   - _Completed — `ensure_seeded` reuses the embedded dataset, though analytics projections still need wiring._
4. **Persistence wiring**
   - Replace `mneme::sqlite::SqliteDb` with the new SeaORM-backed store while still returning `PersistedCommit`/`CommitSummary` DTOs.
   - Ensure `PraxisEngine::with_sqlite` (and the Tauri worker) initialise the SeaORM connection, run migrations/seeds, and expose the async store to the engine.
   - _Completed — the engine now instantiates `SqliteDb`, seeds when empty, and replays commits from the SeaORM-backed store._

### Definition of Done

- Application restart preserves entire history; `list_commits('main')` includes authoritative timestamps.
- `state_at()`/`diff()` replay commits deterministically (with memoised caches) to meet SLOs without depending on blobbed snapshots.
- Docs updated with storage layout & recovery steps; ADR recorded if format is new.

### Testing & Quality

- Async unit tests that exercise the new SeaORM store (store/merge semantics, branch CAS, snapshot tag lifecycles).
- Integration tests to ensure migrations/seeds populate the baseline dataset and Metis tables as expected.
- Property test: `apply(diff(a,b), snapshot(a)) == snapshot(b)` still holds after the async refactor.
- Commands to run before commit: `cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test --all --all-targets`, `pnpm run node:test:coverage`.

### Operational Tooling

- `cargo aideon_xtask health --datastore .praxis` scans branch heads, commit timelines, and `snapshot/<commit>` tags without mutating state and exits non-zero if drift or corruption is detected. Use `--branch` to scope checks to a single scenario and `--quiet` for automation-friendly logging.

### Commit & Tracking Guidance

- Commit 1: Introduce `CommitStore` trait + file-backed implementation.
- Commit 2: Refactor Praxis engine to use store + update tests.
- Commit 3: Add migration tooling + docs.
- Reference GitHub issue; include `Fixes #<id>`; ensure DoD checklist updated via `pnpm run issues:dod`.

### Progress Log

- 2025-11-12 — Added versioned migration history via `mneme_migrations` and regression coverage that reopens SQLite datastores without reapplying schema DDL.
- 2025-11-13 — Converted the Mneme store to async, removed the bespoke Tokio runtime, and updated engine/host toolchains to await persistence operations.
- 2025-11-14 — Persist commit-level analytics into `metis_events` during `put_commit` and covered the projection with an integration test against the SQLite backend.
- 2025-11-15 — Populate `metis_commit_nodes`/`metis_commit_edges` alongside events, index the new tables via migrations, and expand regression coverage to validate the fact payloads.

## WS-B — Meta-Model Enforcement & Configurability

**Objective:** Materialise the ArchiMate-aligned meta-model as machine-readable data, enforce it during commits, and expose it via Typed IPC for UI adaptability.

**Status (2025-11-11):** `docs/data/meta/core-v1.json` is now the canonical schema payload delivered as part of the
baseline dataset; Praxis loads it via `MetaModelRegistry`, and the Tauri command `temporal_metamodel_get`
feeds the renderer’s new Meta-model panel plus the adapter contracts. Overrides remain data-driven to keep
swaps trivial.

### Status

**Partially complete**

- Canonical schema captured in `docs/data/meta/core-v1.json`, loaded by `MetaModelRegistry`, and exposed over IPC so the renderer can display a read-only schema reference.
- Validation now runs inside `GraphSnapshot::apply`, ensuring commits respect type and relationship rules.

**Outstanding**

- Authoring, override merge flows, and schema commits created from the UI are still pending — the current panel is read-only and the registry defaults to the embedded document only.

### Tasks

1. **Schema artefacts**
   - Convert `docs/DESIGN.md` meta-model into a commit-style payload (`docs/data/meta/core-v1.json`), covering graph-style object types, nested attribute blocks, relationships, and plan-event rule definitions.
   - Treat overrides as additional payloads placed alongside the baseline dataset (e.g., `.praxis/meta/<tenant>.json`) so the registry can merge them without code changes.
   - Seed the meta-model through the same initial commit that inserts the first nodes/edges (using the public commit API) rather than importing JSON at runtime so we dogfood the authoring surface.
   - _Baseline payload committed; override ingestion and authoring workflow still need to be implemented._
2. **Registry implementation**
   - Build `MetaModelRegistry` (Praxis crate) to load base + overrides, validate version, and expose APIs for lookup/validation.
   - _Completed — registry materialises descriptors and validation helpers consumed by the engine._
3. **Validation integration**
   - Update `GraphSnapshot::apply` to call registry validators for creates/updates/edges.
   - Emit typed `PraxisError::ValidationFailed` with actionable messages.
   - _Completed — validation executes during snapshot application and surfaces structured errors._
4. **Host exposure & datastore lifecycle**
   - Add Tauri command `temporal_metamodel_get` returning schema metadata.
   - Extend adapters/UI to cache schema and use it for form generation (no backend logic in renderer).
   - Introduce a `mneme::create_datastore` helper so new SQLite stores are created via code: the helper runs during host startup (or when user supplies a name) to prepare the `.praxis/datastore.json` state file, create the named database, and return the path that `PraxisEngine` then opens and seeds.
   - _Completed — IPC command and datastore helper are present; renderer currently consumes the command for read-only display._
5. **Configuration UX**
   - Build renderer screens that let stewards author object types, attributes, and relationships in a nested graph-style view and persist those edits as commit data so the registry sees the new schema immediately.
   - _Pending — the MetaModel panel is read-only and no authoring or commit path exists yet._

### Definition of Done

- Commits violating type/relationship/attribute rules are rejected with deterministic errors.
- Renderer can fetch meta-model snapshot (read-only) and re-render forms without hardcoded enums.
- Schema versioning documented; upgrades require explicit migration notes.

### Testing & Quality

- Rust tests covering valid/invalid nodes, edges, attribute constraints, plan-event rules.
- Snapshot/golden tests for schema serialization.
- Vitest tests verifying adapters parse schema payload and update stores.

### Commit & Tracking Guidance

- Commit 1: Add schema files + registry skeleton.
- Commit 2: Wire validation into graph engine + tests.
- Commit 3: Add host IPC + adapter/UI updates.
- Update `docs/Architecture-Boundary.md` and `docs/DESIGN.md` references.

## WS-C — Baseline Dataset Delivery

**Objective:** Replace toy seed data with the delivered strategy-to-execution dataset aligned with the meta-model, and provide tooling to maintain it.

**Status (2025-11-12):** `docs/data/base/baseline.yaml` now holds dataset v1.0.0, seeded via `PraxisEngine::bootstrap_with_dataset` and the `cargo aideon_xtask import-dataset` CLI (with dry-run validation). Fresh stores receive the meta-model commit plus two baseline commits automatically.

### Status

**Partially complete**

- Authored the first semantic-versioned baseline YAML + changelog, embedded it in Praxis, and documented the import workflow with dry-run validation.
- Added CLI wiring so `cargo aideon_xtask import-dataset` and `ensure_seeded` replay the baseline dataset into new datastores.

**Outstanding**

- Expand dataset coverage (additional commits, scenarios, and larger synthetic fixtures) and add regression checks beyond basic node/edge counts.

### Tasks

1. **Dataset authoring**
   - Model baseline entities/relations in structured files under `docs/data/base/` with semantic version metadata and a change log.
   - _Initial baseline (v1.0.0) is present; further versions and automation for changelog publication remain to be built._
2. **Importer pipeline**
   - `cargo aideon_xtask import-dataset` replays the YAML into ordered commits on `main`, with `--dry-run` enforcing validation in CI.
   - _Completed — the aideon_xtask command supports dry-run/import flows, but CI wiring still needs to be added._
3. **Engine seeding**
   - Praxis `ensure_seeded` now reuses the importer helpers so fresh installs automatically include the meta-model plus baseline commits tagged `baseline/v1`.
   - _Completed — bootstrapping seeds meta-model + baseline when `main` is empty; upgrade path still needs hash verification._
4. **Docs & maintenance**
   - Create `docs/data/README.md` explaining dataset structure, update process, and QA checklist.
   - _Initial README exists; automated linting and stewarding processes remain to be wired into CI._

### Definition of Done

- Fresh installs reproduce baseline dataset; renderer shows real nodes/edges & relationships.
- Dataset updates follow semantic versioning; importer idempotent on empty stores and safely validates via `--dry-run`.
- QA test (`dataset::tests::dataset_bootstrap_yields_expected_counts`) validates counts, mandatory attributes, and cross-links per design.

### Testing & Quality

- Integration test runs importer into temporary store; asserts expected counts per type.
- Lint dataset files via JSON Schema/YAML validation (add to CI) and capture in `docs/data/README.md` workflow.
- Document manual validation steps (e.g., `cargo aideon_xtask import-dataset --dry-run`).

### Commit & Tracking Guidance

- Commit 1: Add dataset files + documentation.
- Commit 2: Add importer CLI + tests.
- Commit 3: Update engine seeding + remove obsolete hard-coded sample.

## WS-D — Reporting & Analytics Functionality

**Objective:** Deliver analytics jobs (Metis) and UI reporting surfaces aligned with design documents (capability scorecards, roadmap timelines, diff/impact views).

### Status

**Not started**

- Contracts exist for analytics jobs, and the renderer shows placeholder metrics, but no backend analytics implementations or UI visualisations have shipped.

**Outstanding**

- Implement Metis analytics jobs, wire host IPC commands, expand renderer surfaces, and add caching/performance guardrails as originally scoped.

### Tasks

1. **Worker jobs**
   - Flesh out `crates/metis` with algorithms for `Analytics.ShortestPath`, `Analytics.Centrality`, `Analytics.Impact`, `Finance.TCO` using graph snapshots.
   - Provide deterministic fixtures and SLO instrumentation.
   - _Not started — `crates/metis/src/lib.rs` still exports a placeholder string._
2. **Host adapters**
   - Expose new Tauri commands (e.g., `analytics_shortest_path`, `report_capability_scorecard`).
   - Update TypeScript adapters/contracts to call commands with typed payloads.
   - _Not started — no Tauri analytics commands or adapter methods exist yet._
3. **Renderer UX**
   - Expand `MainView` (overview tab) with KPI cards, capability heatmap, diff legends, scenario comparisons.
   - Add reporting exports (CSV/PDF placeholder) triggered from UI; ensure PII redaction and scenario context.
   - _Not started — MainView still presents snapshot counts and diff metrics only._
4. **Caching & performance**
   - Cache analytics outputs per commit to meet latency requirements (<100 ms UI updates, <300 ms worker jobs where specified).
   - _Not started — caching and instrumentation await the analytics implementations._

### Definition of Done

- Renderer invokes analytics/reporting commands via IPC; results populate UI + exports without placeholder data.
- Documentation (README/UX guides) includes screenshots and usage instructions.
- PII redaction tests confirm reports contain safe data only.

### Testing & Quality

- Rust unit/integration tests for each analytics job with golden results; record performance metrics in logs.
- Vitest component tests & accessibility checks (capability grid, timeline charts).
- Optional Playwright flow verifying user can run report, download export, and view conflict sets.

### Commit & Tracking Guidance

- Commit per algorithm or UI module to keep diffs reviewable (e.g., `feat(analytics): add shortest path job`).
- Include screenshots/GIFs in PR descriptions for UI changes.
- Update CHANGELOG and ROADMAP milestones as features land.

## WS-E — Process & Quality Guardrails

**Objective:** Institutionalize Definition of Done, CI, and tracking so future work remains aligned with architecture.

### Status

**In progress**

- Consolidated GitHub helper CLI (`scripts/issues.py`) and accompanying npm aliases are available for common workflows.
- `pnpm run ci` covers formatter, lint, typecheck, and unit tests across Node and Rust targets.

**Outstanding**

- Extend automation to cover dataset/schema linting, analytics coverage, and project sync enforcement in CI.
- Author follow-up ADRs for storage, schema, dataset, analytics, and governance changes — only legacy ADRs are present today.

### Tasks

1. **Issue hygiene**
   - Enforce DoD template on all `status/in-progress` issues via `pnpm run issues:dod`.
   - Automate project sync with `pnpm run issues:project` in CI or pre-push hook.
   - _CLI support exists, but enforcement is still manual and needs CI/pre-push integration._
2. **CI coverage**
   - Ensure `pnpm run ci` runs on feature branches; fail fast on lint/test gaps.
   - Add dataset/import + schema validation steps to CI pipeline.
   - _Partial — core lint/test checks run, but dataset/schema validation and analytics coverage jobs are not yet wired in._
3. **Documentation & ADRs**
   - Record key decisions (storage format, schema versioning, reporting architecture) as ADRs under `docs/adr/`.
   - _Pending — current ADR folder only contains legacy entries; new rectification decisions still need to be authored._
4. **Release gating**
   - Update release checklist to include dataset regression, analytics SLO verification, and PII redaction tests.
   - _Pending — supporting automation (e.g., `release:verify`) has not been added to the repository scripts._

### Definition of Done

- Every PR references tracked issue with completed DoD checklist.
- CI pipeline enforces new validation steps and publishes coverage/artifacts needed by Sonar.
- ADRs exist for each architectural change; docs cross-link to plan outcomes.

### Testing & Quality

- Periodic dry-run: `pnpm run ci` locally + `cargo test` before any push.
- Validate hooks (pre-commit, pre-push) stay deterministic and fast.

### Commit & Tracking Guidance

- Commit updates to tooling/docs as separate PRs to simplify review.
- Tag releases once all workstreams’ DoD items close; verify via `pnpm run issues:project`.

## 3. Timeline (Indicative)

| Week       | Focus                                           |
| ---------- | ----------------------------------------------- |
| 1–2        | WS-A storage implementation & migration tooling |
| 3–4        | WS-B meta-model registry + validation           |
| 5–6        | WS-C dataset import + seeding                   |
| 7–8        | WS-D analytics jobs backend                     |
| 9–10       | WS-D reporting UI/exports                       |
| Continuous | WS-E process guardrails, ADRs, CI enhancements  |

Adjust based on issue sizing; break down large tasks using `pnpm run issues:split`.

## 4. Reporting & Governance

- **Status cadence:** Weekly architecture sync reviewing progress per workstream; update plan document as milestones complete.
- **Metrics:** Track `state_at()` latency, schema validation failure rate, dataset import duration, analytics job runtimes.
- **Risk log:** Maintain in `docs/issues/` mirroring GitHub issues for transparency.

## 5. Next Actions

1. Finish the WS-A backlog: convert the store trait to async, add real migrations, and start populating analytics read models.
2. Deliver WS-B configuration workflows so schema overrides can be authored and committed through the product experience.
3. Expand WS-C datasets with additional versions and synthetic fixtures, and wire importer validation into CI.
4. Stand up WS-D analytics end-to-end (worker jobs, host commands, renderer surfaces, caching) before claiming reporting parity.
5. Close the WS-E governance gaps by automating issue hygiene, adding missing ADRs, and extending CI/release gates to cover datasets, schema, and analytics.
