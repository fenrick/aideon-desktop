# AGENTS.md

Guide for AI coding assistants (e.g., Claude, GPT/Codex) contributing to the **Aideon Suite**
repository. The current implementation focus is the **Praxis** desktop module. This is an
**evergreen build**: upgrade to current patterns over preserving legacy seams.

## Purpose

Describe how AI agents should work in this repo: what to read first, how to keep changes small and
safe, and which boundaries and workflows must always be respected. Default to refactoring legacy
code toward the modern stack instead of extending old seams.

Before making changes, agents should read:

- Root `README.md` (suite overview and modules)
- `docs/DESIGN.md` (suite-level design and principles)
- `ARCHITECTURE-BOUNDARY.md` (layers, adapters, time-first boundaries)
- `docs/CODING-STANDARDS.md` (coding rules and boundaries)
- `docs/TESTING-STRATEGY.md` (testing expectations)
- The `README.md` and `DESIGN.md` for the module they are working in

## Documentation index

- **Canonical**: `README.md`, `docs/DESIGN.md`, `ARCHITECTURE-BOUNDARY.md`, `docs/CODING-STANDARDS.md`, `docs/TESTING-STRATEGY.md`, `docs/DESIGN-SYSTEM.md`, `docs/UX-DESIGN.md`, and module-level `README.md` + `DESIGN.md`.
- Legacy Svelte renderer (`app/PraxisDesktop/`) has been removed; ignore any remaining references to it.

> **Scope:** These instructions apply exclusively to the `aideon-praxis` codebase. Do not spend time optimising for downstream consumers, SDKs, or hypothetical adopters outside this repository unless explicitly directed in a task.

## Who this is for

- AI pair‑programmers generating or refactoring code, tests, and docs.
- Tools that open pull requests or propose patches.

## Core principles (do not break)

- **Time-first digital twin:** time context (valid time + asserted time), scenarios, plan/actual layers,
  and Plan Events are authoritative.
- **Local‑first, cloud‑ready:** Desktop works offline; server mode is a config switch, not a fork.
- **Strict boundaries:** Renderer calls Host via Tauri invoke (typed adapters). Desktop mode runs engines in-process. Host calls engines via Rust traits. No sockets. No DB‑specific logic in the renderer.
- This repository’s documentation defines the target architecture; code is updated to match it.
- **Security by default:** No renderer HTTP; no open TCP ports in desktop mode; PII redaction on
  exports; least privilege.
- **Adapters, not entanglement:** Graph, Storage, Worker are interface‑driven. Backends swap without
  UI change.
- **Evergreen:** Treat older patterns as candidates for upgrade; current suite and module design docs
  take precedence over legacy notes. Favour refactoring toward the current stack instead of extending
  legacy seams.

## UX Shell (Aideon Desktop)

- When building or modifying application-level layout, target the Aideon Desktop shell instead of giving individual workspaces their own chrome.
- Use design-system proxies for Sidebar, Resizable, and Menubar/Toolbar primitives; do not implement custom layout primitives.

## Frameworks-first defaults (use these before inventing your own)

- **TS/React:** React 19, shadcn/ui + Tailwind, React Flow/XYFlow for canvases, React Hook Form for
  forms, Testing Library + Vitest for tests, pnpm 10, Node 24. Reach for TanStack Table when you
  need tables; avoid bespoke component primitives.
- **Rust:** tokio for async, serde for serialization, thiserror for typed errors, tracing + `log`
  facade for logging, dirs/directories for platform paths, anyhow for internal glue only,
  serde_json/bincode as defaults before adding new formats. Prefer established crates over custom
  helpers.

Do not build your own UI kits, form/state helpers, logging wrappers, or async executors unless an
design docs require it.

## Repository boundaries (monorepo)

High-level module boundaries are documented in `ARCHITECTURE-BOUNDARY.md` and in each module’s
`README.md`/`DESIGN.md` (see the “Aideon Suite modules” table in `README.md`). Never cross those
boundaries with imports or side-effects (e.g., no renderer ↔ DB access, no engines importing Tauri).

## Evergreen environment & legacy handling

This repository is an evergreen, fast-evolving codebase. Code, docs, and patterns are continuously improved and may change frequently. When resolving conflicts or ambiguity, use the following order of precedence:

- **1. Code on `main`** is always authoritative.
- **2. Suite-level docs:** `docs/DESIGN.md`, `ARCHITECTURE-BOUNDARY.md`, `docs/CODING-STANDARDS.md`, `docs/TESTING-STRATEGY.md`, `docs/ROADMAP.md`.
- **3. Module docs:** `<module>/README.md`, `<module>/DESIGN.md`.
- **4. Supporting docs in `docs/`** (not listed above).
- **Anything else** is informational only and does not override the above.

**Behaviour rules:**

- Prefer updating to current patterns over preserving legacy. When you touch code, refactor it toward the current architecture and coding standards.
- Clean as you go: when modifying a file, remove or clearly mark obsolete TODOs, comments, and dead code in the area you touch, if safe and reasonable.
- Do not add new features on top of obviously legacy seams (e.g., pre-refactor patterns, deprecated UIs). Only maintain or migrate these; do not extend them.
- After changing behaviour or design, update the relevant module `README.md` and `DESIGN.md`. Remove or archive outdated doc sections rather than adding conflicting ones.
- Minimise new `.md` files: only create new module `README.md`/`DESIGN.md` docs. Prefer updating existing docs.

## Checklists

**Before you start a task**

- Skim the docs listed at the top for your area.
- Identify the target module (`app/AideonDesktop/src/canvas`, host crate, engine crate, etc.) and open its
  `README.md`/`DESIGN.md`.
- If you touch legacy code, plan to migrate toward the current stack where safe.
- Locate existing adapters/APIs to reuse; do not add new IPC/HTTP surfaces without need.

**Before you submit changes**

- Run relevant tests/lints: `pnpm run node:test` (or scoped), `pnpm run node:typecheck`, `pnpm run
host:lint && pnpm run host:check`, `cargo test --all --all-targets` as applicable.
- Check coverage impact (goal ≥80% on new code) and note any gaps.
- Update docs you touched (module `README`/`DESIGN`) and this file if behaviours
  changed.
- Re-verify boundaries and security: no renderer HTTP, no new ports, renderer uses typed IPC only.

**Coverage guardrails (run before proposing changes)**

- Always run: `pnpm run node:test:coverage` and `pnpm run host:coverage` (requires `cargo-llvm-cov`).
- If coverage drops below thresholds, add tests or refactor until it passes (TS/React ≥80% lines/branches/functions/statements; Rust engine ≥90%, host ≥80%).

**When touching a boundary (Rust ↔ host ↔ renderer)**

- Update/validate DTO types on both sides (TS in `app/PraxisDtos`, Rust in `crates/mneme`).
- Update `docs/CONTRACTS-AND-SCHEMAS.md` when schemas or IPC error shapes change.
- Ensure error structures are documented and consistent across layers before merging.

**When adding or changing UI components (Praxis Canvas)**

- Copy the golden pattern from the time cursor + temporal panel stack: hooks expose `[state, actions]`, IPC via `praxis-api.ts`, shadcn cards for layout, alerts/skeletons for loading/error.
- Use design-system components directly; avoid bespoke wrappers.
- Ensure loading/error/empty states are covered by tests; mock IPC at the boundary.

**When adding engine/host functionality**

- Follow the patterns in `crates/praxis/DESIGN.md` and `crates/desktop/DESIGN.md` (errors via `PraxisError`/`HostError`, logging with `log`/`tracing`, datastore via Mneme helpers).
- Use `crates/desktop/src/temporal.rs` and `crates/praxis/tests/merge_flow.rs` as golden paths for command wiring and engine flows.

## Task menu for agents (allowed)

- Scaffolding modules, views, adapters, or worker jobs inside the correct package.
- Implementing time‑slicing UI (AS‑OF slider), Plan Event handling, plateau/diff exports.
- Analytics in the Rust worker crates (Chrona/Metis) with tests and metrics.
- Connectors via Continuum scheduler (e.g., CMDB), CSV wizard features, PII redaction,
  encryption‑at‑rest.
- Docs: module `README.md`/`DESIGN.md`, global README, `docs/DESIGN.md`, `ARCHITECTURE-BOUNDARY.md`,
  ROADMAP, C4 diagrams-as-code.

## Examples (golden patterns)

- Desktop shell composition: `app/AideonDesktop/src/design-system/src/desktop-shell/DesktopShell.tsx` and `app/AideonDesktop/src/root.tsx`.
- Workspace navigation: `app/AideonDesktop/src/DesktopTree.tsx` (scenarios → workspaces).
- Selection plumbing: `app/AideonDesktop/src/canvas/app.tsx` (emits `SelectionState`) + `app/AideonDesktop/src/DesktopPropertiesPanel.tsx`.
- Chrome-free canvas surface: `app/AideonDesktop/src/canvas/app.tsx` exported as `PraxisCanvasSurface`.

## Issues, PRs & tracking

### GitHub as source of truth

- Source of truth is GitHub issues and a Projects v2 board. Local Markdown under `docs/issues/` is a mirror only.
- Use the provided CLI helpers (backed by `gh`) to keep tracking tight:
  - `pnpm run issues:start <#>`: assign yourself, add `status/in-progress`, create local branch `issue-#/slug`, sync to Project, mirror docs.
  - `pnpm run issues:split <parent#> --items "Subtask A" "Subtask B"` (or `--file tasks.txt`): creates linked secondary issues, updates parent checklist, syncs/mirrors.
  - `pnpm run issues:project` (or `:dry`): ensure all repo issues are on the configured Project and set its `Status` from labels per `.env` mapping.
  - `pnpm run issues:dod`: ensures a “Definition of Done” section exists on all `status/in-progress` issues.
  - `pnpm run issues:linkify`: comments on issues with links to recent PRs.
  - `pnpm run issues:backfill [--since YYYY-MM-DD] [--close]`: comments on issues referenced in commits on `main`; with `--close` will close issues referenced by Fixes/Closes/Resolves.
  - `pnpm run issues:mirror`: refresh local docs/issues from GitHub; pre‑push enforces freshness via `issues:mirror:check`.

### Environment

Remember that this is a desktop application: everything runs inside packaged binaries on Windows/macOS/Linux, so you must resolve settings and state paths via the platform conventions (AppData, Application Support, XDG directories) or Tauri-provided helpers. Do not assume arbitrary files can be created next to the binary; rely on the APIs that expose the correct directories for config/state instead of hardcoding repo-relative paths once delivered.

The Tauri stack already ships with helpers that are safe to use in these environments: the `tauri-plugin-fs` plugin (file system helpers), `tauri-plugin-dialog` (choose files/directories, prompts), `tauri-plugin-window-state` (persist size/position), etc. Reach for these plugins instead of rolling your own file handling when wiring renderer/host logic so you benefit from the packaged, multiplatform behavior they expose.

Copy `.env.example` to `.env` and set:

- `AIDEON_GH_REPO=owner/repo`
- `AIDEON_GH_PROJECT_OWNER=<org_or_user>`
- `AIDEON_GH_PROJECT_NUMBER=<number>`
- `AIDEON_GH_STATUS_FIELD=Status`
- `AIDEON_GH_STATUS_MAP={"status/todo":"Todo","status/in-progress":"In Progress","status/blocked":"Blocked","status/done":"Done"}`

Token scopes required: `repo`, `project`, `read:project`, and `read:org` if the project is under an organization. Token may be stored in `.env` (not committed). The `.aideon/` local cache and mirrors are ignored via `.gitignore`.

### Definition of Done (DoD)

For any item labeled `status/in-progress`, ensure the issue body contains this section (added via `pnpm run issues:dod`):

- CI: lint, typecheck, unit tests updated
- Docs: user & dev docs updated (README/CHANGELOG)
- Security: renderer IPC boundaries respected; no new ports
- Performance: SLO notes or benches if applicable
- UX: matches GitHub‑inspired style (light/dark)
- Packaging: macOS build verified (DMG/ZIP)
- Tracking: PRs linked; Project Status updated; local mirror refreshed

When finishing work, follow the DoD and workflow expectations in `CONTRIBUTING.md` (including labels,
milestones, and PR linkage).

## Output contract (must follow in every proposal/PR)

Provide your response in the following sections, in this order. Keep explanations concise.

### PLAN

- Bulleted list of files to create/modify, with one‑line reasons.

### TESTS

- What tests you add/modify, how to run them, and expected assertions.

### RUN

- Commands to build/test/lint locally (TS + Rust). Include any data generation steps.

### CHECKS

- Security: confirm no renderer HTTP, no new network ports, PII redaction respected.
- Boundaries: confirm no backend logic in renderer; worker uses RPC only.
- Performance: note expected impact; reference SLOs if relevant.

### NOTES

- Trade‑offs, alternatives rejected, follow‑ups (issues to file).

If you need input, first emit a **short PLAN with questions**; otherwise proceed with sensible
defaults consistent with this guide.

## Coding standards

For coding standards (quality gates, coverage targets, tooling, and CI rules), see
`docs/CODING-STANDARDS.md`. For testing expectations, see `docs/TESTING-STRATEGY.md`.

## Per-module guidance (where to look)

- **Aideon Desktop (flattened) (`app/AideonDesktop`)**
  - Read: `app/AideonDesktop/DESIGN.md`, canvas/docs under `app/AideonDesktop/docs/*` (canvas, adapters, dtos, design system).
  - Contains the React canvas, design-system proxies, adapters, and DTOs in one package.
  - Tests: JS/TS tests via `pnpm run node:test` (Vitest).

- **Aideon Host (`crates/desktop`)**
  - Read: `crates/desktop/README.md`, `crates/desktop/DESIGN.md`, `ARCHITECTURE-BOUNDARY.md`.
  - Constraints: no renderer HTTP; no open ports in desktop mode; typed commands only.
  - Tests: Rust tests via `cargo test -p aideon_desktop`; workspace checks via `pnpm run host:lint && pnpm run host:check`.

- **Engines (`crates/praxis`, `crates/chrona`, `crates/metis`, `crates/continuum`, `crates/mneme`)**
  - Read: each crate’s `README.md`, `DESIGN.md` (where present), `docs/DESIGN.md`, `ARCHITECTURE-BOUNDARY.md`.
  - Constraints: no Tauri or UI dependencies; obey time-first commit model and adapter boundaries.
  - Tests: crate-level `cargo test -p <crate>` plus workspace Rust checks.

## Technology & testing expectations

### TypeScript / React (Praxis Canvas, app/PraxisAdapters)

– Node 24, React 19. Strict TS config; ESLint + Prettier. All new surface/canvas work targets the
React + React Flow + shadcn/ui stack
described in `docs/UX-DESIGN.md`, `docs/DESIGN-SYSTEM.md`, and
`app/AideonDesktop/docs/praxis-canvas/DESIGN.md`.

- Tauri renderer: no Node integration; `contextIsolation: true`; strict CSP; capabilities restrict
  plugin access. The host exposes typed commands only, and React components call the host through a
  dedicated `praxisApi` wrapper rather than ad-hoc IPC.
- For app shell layout, always use the design-system proxies for Sidebar, Resizable, Menubar, and
  Toolbar instead of importing raw shadcn or react-resizable-panels primitives.
- Never embed backend‑specific queries in renderer; call adapters or host APIs. React Flow widgets
  must treat the twin as the source of truth.

### Lint/Format discipline and code quality

- Do not disable lint rules in code (no inline `eslint-disable`, `ts-ignore`, etc.) in new React/
  TypeScript modules.
- Refactor code to satisfy linters and static analysis rather than suppressing warnings.
- Use check-only hooks locally; CI enforces the same rules.
- Coverage targets (Node/TS and Rust): Lines ≥ 80%, Branches ≥ 80%, Functions ≥ 80% on new code; overall should trend upward.
- Sonar: `sonar.new_code.referenceBranch=main` configured; CI waits for the Sonar Quality Gate before merging.
- Keep code paths single and explicit. Desktop mode runs engines in-process. Host calls engines via Rust traits. No sockets.
  - It is acceptable to expose test-only helpers (e.g., `__test__`) to raise branch coverage when they don’t affect runtime. For React widgets, add Vitest + Testing Library smoke tests alongside the new runtime as soon as it exists.

### Rust worker crates (crates/chrona, crates/metis, crates/praxis, crates/continuum)

– Rust 2024 edition, `cargo fmt` + `cargo clippy --all-targets --all-features` clean.

- Keep execution logic behind traits so adapters can swap local vs remote implementations.
- Prefer streaming-friendly payloads (Arrow/bytes) over large JSON blobs when adding new APIs.

### Docs

- Markdown, markdownlint clean (no heading jumps; 2‑space nested bullets).
- Diagrams‑as‑code preferred (Structurizr DSL, Mermaid, PlantUML) stored under `docs/c4/`.
- When updating docs, prefer editing existing suite/module docs; avoid creating new `.md`
  files unless they are a new module `README.md`/`DESIGN.md`.

## Contracts snapshot (reference only)

The current worker jobs and time APIs are defined by engine contracts and module design docs:

- Worker job types and payloads: see Metis and temporal engine design docs.
- Time APIs and PlanEvent schema: see Praxis engine design docs.

- `Analytics.Centrality { algorithm: degree|betweenness, scope }`
- `Analytics.Impact { seedRefs[], filters{} }`
- `Temporal.StateAt { asOf, scenario?, layer? }`
- `Temporal.Diff { from: plateauId|date, to: plateauId|date, scope? }`
- `Temporal.TopologyDelta { from, to }`
- `Finance.TCO { scope, asOf, scenario?, policies? }`

### Time APIs (desktop read‑only; server read/write)

- `GET /graph?as_of=YYYY‑MM‑DD&scenario=&confidence=`
- `GET /diff?from=...&to=...`
- `GET /topology_delta?from=...&to=...`
- `GET /tco?scope=...&as_of=...&scenario=...`

### Minimal PlanEvent schema (do not change without updating design docs)

- `id, name, effective_at, confidence, source{ work_package?, priority? }, effects[]`
- `effects[]` items: `{ op: create|update|delete|link|unlink, target_ref, payload }`

## Security & privacy rules

- No renderer HTTP; renderer ↔ host via IPC only.
- Desktop mode: no open TCP ports; localhost APIs are host‑bound and read‑only.
- PII: deny‑by‑default on exports/APIs; redaction checked in tests where applicable.
- Do not call external LLMs or telemetry endpoints; if necessary, stub behind host with explicit
  allowlist.

## Performance & SLO gates

- Performance SLOs for temporal APIs and analytics are defined in `docs/ROADMAP.md` (SLO section) and
  in module design docs for engine crates. When you change code that could affect performance, call
  out expected impact in **CHECKS** and add a quick benchmark or test as appropriate.

## Testing guidance

- TS: unit tests for adapters. Renderer calls Host via Tauri invoke (typed adapters). UI state; avoid DOM‑heavy tests unless needed.
- Rust: unit tests/integration tests for host and domain crates (chrona/metis/praxis);
  deterministic seeds for graph generators.
- Golden datasets: provide small synthetic graphs for 5k/50k nodes to assert performance envelopes.
- Add tests for PII redaction and role filtering where functions touch exports.

## CI expectations

- Lint + tests pass for TS and Rust on macOS/Windows/Linux.
- For large algorithms, mark perf tests as optional but runnable locally; capture metrics in logs.
- Coverage gates: verify Node/TS via `pnpm run node:test:coverage` and Rust via
  `cargo test --all --all-targets` with coverage tooling when touching engine logic. Sonar Quality
  Gate must pass.

### Commit Hygiene

- Ensure that code will pass the GitHub CI checks with:
  - `pnpm run ci` (TS/React + Rust lint, typecheck, tests, format).
- Pre‑commit hook: this repo uses Husky to run the above automatically; keep the hook fast and deterministic.

## Versioning & migrations

- Schema is forward‑only. Provide migration scripts and bump `schemaVersion`.
- Snapshots are immutable; scenarios branch and merge; no history rewrites.

## Issues and PR workflow

- Use milestone (M0–M6), labels (`type/*`, `area/*`, `module/*`, `priority/*`).
- Conventional Commits in PR title and commits (e.g., `feat(time): add plateau diff endpoint`).
- Link to updated design docs when changing boundaries, protocols, or the meta-model.

## When to ask vs. when to proceed

- Ask (emit a short PLAN + questions) if:
  - Requirements conflict with guardrails; the change touches security, RPC, or meta‑model.
  - A dependency is ambiguous or unavailable.
- Proceed with defaults if:
  - The task is a local refactor or additive feature within a package and follows this guide.

## Example response template (use this shape)

- Add `Temporal.TopologyDelta` trait to `crates/metis` with empty stub.
- Wire Tauri command to call the new trait via `WorkerState` adapter.
- Extend the React canvas store to request topology delta and render placeholder counts.

PATCH

- Unified diffs here, paths from repo root.

- New unit tests for topology_delta stub in `crates/metis`; Vitest store smoke tests.

RUN

- pnpm run test && pnpm run lint && pnpm run typecheck
- cargo test --all --all-targets

CHECKS

- No renderer HTTP added; no new network ports; adapters unchanged in renderer.
- PII redaction unaffected. Expected p95 unchanged; added memoisation in worker.

NOTES

- Next: add SVG compare export and benchmark on 50k/200k dataset.
