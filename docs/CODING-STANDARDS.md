# Coding Standards (v2)

> Doc update checklist: when changing a public API, a cross-module boundary, or adding a pattern that should become standard, update this file plus the relevant module `DESIGN.md`/`README.md` in the same PR so guidance stays in sync.

## Purpose

Define the shared coding rules, quality gates, and module boundary expectations for the **Aideon Suite**
monorepo. Praxis is the current primary desktop module, but these standards apply to all modules
(Chrona, Metis, Continuum, Mneme) unless explicitly scoped.

- Stacks: Node 24 (TS/React canvas runtime), Rust 2024 (host + engines)
- Monorepo: pnpm workspaces; Cargo workspace for Rust crates
- Runtime posture: typed adapters over IPC; no renderer HTTP and no open TCP
  ports in desktop mode

**Evergreen posture:** code on `main` and current module `DESIGN.md` docs override older notes. Prefer upgrading legacy
areas to the current stack instead of preserving them.

**Frameworks-first defaults:**

- TS/React: React 19, shadcn/ui + Tailwind, React Flow/XYFlow for canvas graphs, React Hook Form for
  forms, Testing Library + Vitest for tests. Use TanStack Table when you need tables; do not build
  bespoke UI primitives if a standard component exists.
- Rust: tokio for async, serde for serialization, thiserror for typed errors, tracing + `log` facade
  for logging, dirs/directories for platform paths; prefer serde_json or bincode before adding new
  formats. Avoid custom logging/async frameworks unless documented in module design.

## Architecture & Boundaries

This document does **not** redefine architecture; it applies coding rules to it.
For the canonical description of layers, adapters, and time-first boundaries across Aideon Suite,
refer to `ARCHITECTURE-BOUNDARY.md`. The notes below focus on product/module naming only.

### Product modules & naming (authoritative)

- **Canonical product modules**: Praxis (core/host & engine orchestration), Mneme (persistence & shared DTOs), Metis (analytics/reasoning), Chrona (time/visualisation), Continuum (automation/orchestration).
- **When to add a new product module** (require ≥2): distinct runtime/container; distinct dependency profile; versioned public API with different release cadence; multi‑consumer reuse across UIs/services; separate security/licensing boundary.
- **Repo‑wide naming prefixes**:
  - Rust crates: `praxis_*`, `mneme_*`, `metis_*`, `chrona_*`, `continuum_*`.
  - TypeScript packages (pnpm workspace): `@aideon/Praxis*`, `@aideon/Mneme*`, `@aideon/Metis*`, `@aideon/Chrona*`, `@aideon/Continuum*`.
- **Approved suffixes** (meaningful roles only): `-core`, `-engine`, `-adapter`, `-sqlite`, `-postgres`, `-api`, `-ui`, `-worker`, `-cli`, `-macros`, `-xtask`.
- **Prohibited names**: `util`, `common`, `shared` (without a clarifying role). Use domain‑specific names or one of the approved suffixes.
- **Path stability**: external import paths must remain stable across refactors; reshape internal structure via facades/barrel exports rather than changing consumer paths.

## Tooling & Performance at Scale

- Caching: enable dependency and build caches in CI for pnpm and Cargo.
- Incrementality:
  - TypeScript: use projectReferences and/or path aliases for workspace packages.
  - Tests: prefer package‑scoped test runs; use changed‑files filters when
    practical.
- Parallelism: run JS/TS and Rust checks in parallel CI jobs where possible.
- Slow tests: quarantine and track with an issue; do not disable gates.

## Quality Gates

- Coverage targets (Node/TS and Rust) on new/changed code:
  - Lines ≥ 80%
  - Branches ≥ 80%
  - Functions ≥ 80%
- Exceptions: generated code and build artifacts are excluded; justify any
  additional exclusions with a comment and an issue reference.
- Flaky tests: quarantine behind a tag and open an issue; fixing flakiness has
  higher priority than adding more tests.
- SonarQube
  - New code is measured against `main` (see `sonar.new_code.referenceBranch`)
  - CI waits for Sonar Quality Gate; failing the gate blocks merges

## TypeScript / React (Praxis Desktop)

- Tooling: ESLint + Prettier; Vitest for unit tests; React compiler/runtime checks. The legacy
  Svelte renderer has been removed; do not introduce new Svelte surfaces.
- Environment: Node 24, pnpm 10; strict TS config
- Rules
  - No inline rule suppressions (no `eslint-disable`, `ts-ignore`, etc.). If absolutely necessary,
    require an issue reference and a TODO explaining removal criteria.
  - No backend‑specific logic in the renderer; IPC only via the typed `praxisApi` bridge. React
    components must not reach for `@tauri-apps/api` directly.
  - Sanitise/validate data crossing the preload boundary; avoid leaking privileged data into
    renderer state. React Flow nodes/edges should carry only the data required for rendering.
- Keep code paths single and explicit; the app speaks to the host over typed IPC commands, and the
  host selects the appropriate engine adapter.
- Coverage
  - Run: `pnpm run node:test:coverage`
  - Add targeted, focused tests; it is acceptable to export clearly named
    test‑only helpers (e.g., `__test__`) to improve branch coverage when it does
    not affect runtime
- Monorepo build health
  - Prefer TS projectReferences and/or path aliases for shared packages
  - Avoid deep imports across package boundaries; publish explicit entrypoints

### TypeScript package & module layout rules

1. **Packages (workspace)**

- One purpose per package; no circular dependencies.
- Keep dependencies minimal; runtime deps belong in runtime packages, dev‑deps in the root or the package using them.
- Do not import from a sibling package’s internal paths; import only from the package root entry (`"exports"`).
- Use the `exports` field to define the public surface; keep internals private.
- Browser code must not depend on Node‑only modules.

2. **Modules & directories**

- One concept per module; if a module gains children, use a directory with an `index.ts` façade.
- Keep nesting shallow (≤3 levels); prefer a flat tree to deep hierarchies.
- Avoid “util”/“misc”; name by domain or role.

3. **Files**

- One responsibility per file; split when a second reason to change appears.
- Keep files short (aim ≤300–500 LOC).
- Co‑locate unit tests as `*.test.ts`/`*.spec.ts`; put integration/e2e tests in dedicated folders (`tests/`).
- Barrel files (`index.ts`) should export only the public API, not everything by default.

4. **Naming**

- Folders/packages: `kebab-case`; files: `kebab-case`.
- Variables/functions: `camelCase`; types/classes/interfaces: `PascalCase`.
- Suffixes by role: `*.service.ts`, `*.repo.ts`, `*.schema.ts`, `*.handler.ts`, `*.adapter.ts`.
- Avoid abbreviations and generic labels.

5. **Visibility & API shaping**

- Prefer **named exports** for libraries; avoid default exports.
- Surface a tidy API via `index.ts` with selective re‑exports; do not leak folder structure.
- Do not deep‑import across packages (`pkg/internal/…`).

6. **Layering & boundaries**

- Renderer is UI only; no Node APIs—communicate via typed IPC.
- Host orchestrates side effects; business logic shared across layers belongs in a pure library package.
- Define ports (TypeScript interfaces) for external systems; implement adapters in infra packages.
- DTOs (I/O shapes) are separate from domain models.

7. **Dependencies**

- Avoid leaking third‑party types in public APIs; wrap at the boundary.
- Prefer small, well‑maintained libraries; pin versions where stability matters.
- Keep test‑only deps as dev‑deps; avoid bringing heavy test deps into runtime packages.

8. **Errors**

- Throw `Error` subclasses with a stable `code` and optional `cause`; never throw strings.
- Map external/library errors at the boundary; do not expose raw error shapes across package APIs or IPC.

9. **Configuration**

- Validate environment/config with a schema (e.g., Zod) at startup; crash fast on invalid config.
- Read `process.env` only in one composition layer; pass typed config downward.
- Use build‑time flags for environment branches (`import.meta.env`, `$app/environment`); avoid ad‑hoc globals.

10. **Async & state**

- Prefer pure, stateless functions; isolate side effects.
- Use `AbortSignal` for cancelable async work; ensure cleanup in finally blocks.
- Avoid module‑level mutable state in libraries; if needed, hide behind a factory and document lifecycle.

11. **Feature flags / conditionals**

- Centralise feature flags; do not scatter `if (process.env…)` throughout the codebase.
- Keep flags orthogonal and documented; avoid changing public types under flags.

12. **Testing**

- Unit‑test behaviour via public exports; minimise reliance on private internals.
- Use fakes over heavy mocks for ports/adapters; keep tests deterministic and isolated.
- Snapshot tests only for stable, intentional outputs (e.g., rendered HTML fragments, schemas).

13. **Documentation & comments**

- Start each module with a brief JSDoc/TSDoc explaining purpose and invariants.
- Document all public exports succinctly; include small examples where helpful.

14. **Formatting, lints & CI**

- Enforce ESLint (no inline suppressions without an issue link) and Prettier.
- Forbid deep imports across packages via lint rules; enforce import ordering.
- CI fails on lint/type errors and on coverage gates for changed code.

15. **Refactors & history**

- Apply changes incrementally; preserve history when moving files.
- After splits, re‑shape each package’s `index.ts` so consumer import paths stay stable.

## Rust (Host & Engine Crates)

- Tooling: Cargo (`cargo fmt`, `cargo clippy`, `cargo test`)
- Environment: Rust stable (rustup manages toolchain + components)
- Commands
  - `pnpm run host:format` / `host:format:check` — `cargo fmt`
  - `pnpm run host:lint` — `cargo clippy --all-targets -- -D warnings`
  - `pnpm run host:check` — `cargo check` across the workspace
  - `cargo test --all --all-targets` — unit/integration tests for host and
    engine crates
- Rules
  - Engines expose typed traits; keep adapters thin and deterministic
  - No open TCP ports in desktop mode; remote adapters must stay behind the
    same trait boundaries
  - Prefer fast, deterministic tests; add integration tests when touching IPC
    layers or async flows

### Rust crate & module layout rules

1. **Crates**

- Each crate has one clear purpose and one reason to change.
- No cyclic dependencies. Put shared traits/types/errors in a small core crate; backend/impl crates depend on it.
- Keep dependencies minimal; isolate heavy/optional deps in leaf crates.
- Treat crate boundaries as public APIs; keep surfaces small and stable.
- Use features to toggle optional backends or integrations; features are additive and off by default.
- Do not expose third‑party types in public APIs if avoidable.

2. **Modules & directories**

- One concept per module. If a module gains children, switch to a directory with a concise `mod.rs` façade.
- Keep nesting shallow (≤3 levels). Prefer a flat tree to deep hierarchies.
- Avoid “util”/“misc” modules; name by domain or role.

3. **Files**

- One responsibility per file; split when a second reason to change appears.
- Keep files short (aim ≤300–500 LOC).
- Place unit tests alongside the code under `#[cfg(test)]`; put integration tests in `tests/`.
- Prefer file‑level `#[cfg(feature = "...")]` over scattered fine‑grained cfgs.

4. **Naming**

- Use `snake_case`. Choose descriptive, stable names.
- Nouns for data types; role suffixes for behaviour (e.g., `_repo`, `_service`, `_handler`).
- Avoid abbreviations and generic labels.

5. **Visibility & API shaping**

- Default to private. Escalate to `pub(super)` or `pub(crate)` before `pub`.
- Present a tidy API via facades (`lib.rs`/`mod.rs`) with selective `pub use`.
- Keep internal structure hidden; callers see a clean, flat surface.

6. **Layering & boundaries**

- Separate domain, persistence, transport, and presentation concerns.
- Do not cross‑leak types between layers (e.g., DB rows never appear in HTTP DTOs).
- Cross layers via traits/interfaces defined in the core.

7. **Dependencies**

- Do not leak implementation‑specific types in public function signatures.
- Keep optional or heavyweight deps behind features and out of core crates.

8. **Errors**

- Provide a crate‑local error type (`thiserror`) and convert foreign errors at the boundary.
- Avoid `anyhow::Error` in public APIs; fine for internals and binaries.

9. **Configuration & initialisation**

- Define typed config structs; deserialise once at the edge.
- Initialise resources (pools, loggers, caches) in the binary/composition layer; pass handles explicitly.

10. **Concurrency & state**

- Prefer explicit, clonable handles (`Arc`, connection pools).
- No global mutable state; if a singleton is required, keep it crate‑private and well‑documented.

11. **Features & conditional compilation**

- Keep features orthogonal and well‑documented; avoid feature combos that change public types.
- Use features to select backends (e.g., sqlite/postgres) without altering call‑sites.

12. **Testing**

- Test behaviour through public APIs and traits; minimise reliance on private internals.
- Provide lightweight in‑memory or fake backends for logic tests.
- Keep tests deterministic and isolated; no shared mutable state across tests.

13. **Documentation & comments**

- Start each module/file with a brief `//!` explaining purpose and invariants.
- Document all `pub` items succinctly with examples where helpful.

14. **Formatting, lints & CI**

- Enforce `rustfmt` defaults and `clippy` (at least `-W clippy::all`); fail CI on warnings in library crates.
- Keep imports ordered; avoid wildcard imports in libraries.

15. **Refactors & history**

- Apply changes incrementally; preserve history with `git mv`.
- After splits, re‑shape the façade so external paths stay stable.

### Rust crate & module naming conventions

- **Crate package names** (Cargo `[package].name`): kebab-case (e.g., `praxis`); the crate library name defaults to snake_case; set `lib.name` only if needed for coherence.
- **Prefixes & suffixes**: use the product prefixes from _Product modules & naming_ and the approved suffixes. Proc‑macro crates end with `-macros`; the automation crate at the workspace root is named `aideon_xtask`.
- **Modules/files**: snake_case; prefer clear domain nouns. Avoid abbreviations and generic labels. Only create a `prelude` module for a small set of frequently used traits/types; keep it minimal and opt‑in.
- **Features**: all‑lowercase, additive; backend features (`sqlite`, `postgres`, `memory`), platform features (`desktop`, `server`). Features must not change public type shapes in incompatible ways.
- **Public API paths**: expose a flat, tidy surface via `lib.rs` facades and selective `pub use`. Do not leak third‑party types or deep module paths in public signatures.

## Commit Hygiene

Run these locally before every commit (CI enforces the same):

- App: `pnpm run node:format` → `pnpm run node:lint:fix` → `pnpm run node:typecheck` → `pnpm run node:test:coverage`
- Host/engines: `pnpm run host:format` → `pnpm run host:lint` → `pnpm run host:check` → `cargo test --all --all-targets`
- Coverage gates must be met on new/changed code in both app and Rust crates
- No rule suppressions; refactor to satisfy linters and analyzers
- Conventional Commits are required; use `feat|fix|chore|ci|docs|test|refactor|perf|style`
- Pre‑commit hooks should run only on changed files; use `--no-verify` only with
  justification and follow‑up issue

## CI & Sonar

- CI uses pnpm 10 and Cargo; ensure `rustfmt` and `clippy` components are
  installed in the build environment
- Sonar runs Clippy as part of Rust analysis; keep the CI environment’s Rust toolchain
  consistent and include `clippy` to avoid scan-time failures.
- Sonar analyzes both Node/TS and Rust; coverage reports are uploaded from
  Vitest (lcov) and Rust coverage tooling (lcov). CI waits for the Sonar Quality Gate
- Fail‑fast ordering in CI: lint/type before unit tests; heavier jobs (coverage,
  integration) after quick gates pass
- Cache pnpm/Cargo artifacts; prefer parallel jobs for app and host/engine checks

## Security & Boundaries

- No renderer HTTP; renderer ↔ host via IPC only
- Desktop: no open TCP ports; host ↔ engine adapters stay in-process and typed
- PII: redact by default on exports/APIs; add tests where applicable
- Dependencies: review regularly; avoid unvetted libraries; lock through pnpm
  and Cargo; prefer pinned dev‑tool versions
- Secrets: never hard‑code; provide via env/config; CI scans for secrets
- Threat modeling: required for new public interfaces or boundary changes

## Version & Governance

- Ownership: standards are owned by maintainers; propose changes via PR tagged
  `docs(standards)`.
- Versioning: this document is versioned; record notable changes in PRs and
  changelog.
- Exceptions: document and time‑limit any deviation with issue links and a plan
  to converge.

## References

- Commands: `docs/COMMANDS.md`
- Practices & guardrails: `AGENTS.md`
- Contributor workflow: `CONTRIBUTING.md`
