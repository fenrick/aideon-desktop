# Aideon Suite

This repository contains **Aideon Suite**, a local-first, graph-native digital twin platform that
makes **time and scenario context first-class**. The suite is built as a set of modules that share a
common metamodel, adapter boundaries, and security posture.

Key differentiators:

- **Time-first facts:** bi-temporal facts (valid time + asserted time) with plan/actual layers and
  scenario overlays, not static snapshots.
- **Task + artefact workflow:** users author through tasks and consume stable artefacts (views,
  catalogues, matrices, maps, reports) with integrity and explainability.
- **Secure desktop platform:** a native, offline-first shell with typed IPC only (no renderer HTTP,
  no open ports in desktop mode), ready to pivot to server mode behind adapters.

Within the suite:

- **Aideon Praxis** defines the enterprise metamodel, task-oriented authoring, and artefact outputs.
- **Aideon Chrona** provides temporal visualisation and time UX support.
- **Aideon Metis** focuses on analytics and reasoning.
- **Aideon Continuum** handles orchestration and automation.
- **Aideon Mneme** owns bi-temporal persistence, schema storage, and projection edges.

See `docs/DESIGN.md` for suite-level product and conceptual design, and
`ARCHITECTURE-BOUNDARY.md` for code-level layering and boundaries.

## Aideon Suite modules

The table below lists the primary modules in this repo. See each module's README for details.

| Name                   | Path                | Responsibility                                                                              | Type           |
| ---------------------- | ------------------- | ------------------------------------------------------------------------------------------- | -------------- |
| Aideon Desktop         | `app/AideonDesktop` | React/Tauri desktop shell containing canvas, design system, adapters, and DTOs (flattened). | Node/React app |
| Aideon Host            | `crates/desktop`    | Tauri desktop host exposing typed commands and capabilities.                                | Rust crate     |
| Praxis Engine          | `crates/praxis`     | Metamodel + task APIs, artefact execution, integrity checks, analytics orchestration.       | Rust crate     |
| Chrona Visualisation   | `crates/chrona`     | Temporal visualisation helpers and time/scenario UX support.                                | Rust crate     |
| Metis Analytics        | `crates/metis`      | Analytics jobs (centrality, impact, TCO, rankings).                                         | Rust crate     |
| Continuum Orchestrator | `crates/continuum`  | Scheduler/connectors and background orchestration.                                          | Rust crate     |
| Mneme Core             | `crates/mneme`      | Bi-temporal storage engine (op log, facts, schema, projections) across SQL backends.        | Rust crate     |

For module-level internal design, see each `<module>/DESIGN.md` (where present).

## Getting started

For a full walkthrough (prerequisites, setup, dev workflow, and issues helpers), see
`docs/GETTING-STARTED.md`. The commands below are the most common entry points.

### Common commands (quick reference)

- Install deps: `corepack enable && pnpm install`
- Dev (Praxis workspace + Tauri host): see `docs/GETTING-STARTED.md` for the recommended terminal layout.
- Lint/typecheck/test (TS): `pnpm run node:lint && pnpm run node:typecheck && pnpm run node:test`
- Rust checks: `pnpm run host:lint && pnpm run host:check`

See `docs/GETTING-STARTED.md` and `docs/COMMANDS.md` for the full list of pnpm commands used across
JS/TS and the Rust workspace.

## Key docs

- Suite design: `docs/DESIGN.md`
- Architecture and layering: `ARCHITECTURE-BOUNDARY.md`
- Coding standards: `docs/CODING-STANDARDS.md`
- Testing strategy: `docs/TESTING-STRATEGY.md`
- Agent guidance: `AGENTS.md`
- Roadmap: `docs/ROADMAP.md`
- Aideon Desktop shell: `app/AideonDesktop/DESIGN.md`

For contributing guidelines, see `CONTRIBUTING.md` and `CODE-OF-CONDUCT.md`. The license for this
repo is described in `LICENSE`.
