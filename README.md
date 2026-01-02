# Aideon Suite

This repository contains **Aideon Suite**, a local-first, graph-native digital twin platform that
treats **time as a first-class dimension**. The suite is built as a set of modules that share the
same time-first meta-model, adapter patterns, and security posture.

Within the suite:

- **Aideon Praxis** is the core desktop digital twin module (React/Tauri canvas + Rust engines).
- **Aideon Chrona** provides time-based visualisation.
- **Aideon Metis** focuses on analytics and reasoning.
- **Aideon Continuum** handles orchestration and automation.
- **Aideon Mneme** owns persistence and shared DTOs.

See `docs/DESIGN.md` for suite-level product and conceptual design, and `Architecture-Boundary.md`
for code-level layering and boundaries.

## Aideon Suite modules

The table below lists the primary modules in this repo. See each module’s README for details.

| Name                   | Path                   | Responsibility                                                                              | Type           |
| ---------------------- | ---------------------- | ------------------------------------------------------------------------------------------- | -------------- |
| Aideon Desktop         | `app/AideonDesktop`    | React/Tauri desktop shell containing canvas, design system, adapters, and DTOs (flattened). | Node/React app |
| Aideon Host            | `crates/desktop`       | Tauri desktop host exposing typed commands and capabilities.                                | Rust crate     |
| Praxis Engine          | `crates/praxis`        | Core time-aware graph/commit engine for the digital twin.                                   | Rust crate     |
| Praxis Facade          | `crates/aideon_praxis` | Facade and orchestration layer over Praxis engine and adapters.                             | Rust crate     |
| Chrona Visualisation   | `crates/chrona`        | Temporal visualisation and `state_at`/`diff` helpers.                                       | Rust crate     |
| Metis Analytics        | `crates/metis`         | Analytics jobs (shortest path, centrality, impact, TCO).                                    | Rust crate     |
| Continuum Orchestrator | `crates/continuum`     | Scheduler/connectors and snapshot/layout persistence orchestration.                         | Rust crate     |
| Mneme Core             | `crates/mneme`         | Persistence layer (SQLite/other) and shared commit/ref/snapshot DTOs                        | Rust crate     |

For module-level internal design, see each `<module>/DESIGN.md` (where present).

## Getting started

For a full walkthrough (prerequisites, setup, dev workflow, and issues helpers), see
`docs/getting-started.md`. The commands below are the most common entry points.

### Common commands (quick reference)

- Install deps: `corepack enable && pnpm install`
- Dev (Praxis workspace + Tauri host): see `docs/getting-started.md` for the recommended terminal layout.
- Lint/typecheck/test (TS): `pnpm run node:lint && pnpm run node:typecheck && pnpm run node:test`
- Rust checks: `pnpm run host:lint && pnpm run host:check`

See `docs/getting-started.md` and `docs/commands.md` for the full list of pnpm commands used across
JS/TS and the Rust workspace.

## Key docs

- Suite design: `docs/DESIGN.md`
- Architecture and layering: `Architecture-Boundary.md`
- Coding standards: `docs/CODING_STANDARDS.md`
- Testing strategy: `docs/testing-strategy.md`
- Agent guidance: `AGENTS.md`
- Roadmap: `docs/ROADMAP.md`

For contributing guidelines, see `CONTRIBUTING.md` and `CODE_OF_CONDUCT.md`. The license for this
repo is described in `LICENSE`.
