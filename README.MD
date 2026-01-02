# Aideon Suite

This repository contains **Aideon Suite**, a local-first, graph-native digital twin platform that
makes **time and scenario context first-class**. The suite is built as a set of modules that share a
common metamodel, adapter boundaries, and security posture.

Within the suite:

- **Aideon Praxis** defines the enterprise metamodel, task-oriented authoring, and artefact outputs.
- **Aideon Chrona** provides temporal visualisation and time UX support.
- **Aideon Metis** focuses on analytics and reasoning.
- **Aideon Continuum** handles orchestration and automation.
- **Aideon Mneme** owns bi-temporal persistence, schema storage, and projection edges.

See `docs/DESIGN.MD` for suite-level product and conceptual design, and
`ARCHITECTURE-BOUNDARY.MD` for code-level layering and boundaries.

## Aideon Suite modules

The table below lists the primary modules in this repo. See each module's README for details.

| Name                   | Path                   | Responsibility                                                                                | Type           |
| ---------------------- | ---------------------- | --------------------------------------------------------------------------------------------- | -------------- |
| Aideon Desktop         | `app/AideonDesktop`    | React/Tauri desktop shell containing canvas, design system, adapters, and DTOs (flattened).   | Node/React app |
| Aideon Host            | `crates/desktop`       | Tauri desktop host exposing typed commands and capabilities.                                  | Rust crate     |
| Praxis Engine          | `crates/praxis`        | Metamodel + task APIs, artefact execution, integrity checks, analytics orchestration.         | Rust crate     |
| Praxis Facade          | `crates/aideon_praxis` | Facade/orchestration layer that binds Praxis to host/server adapters.                          | Rust crate     |
| Chrona Visualisation   | `crates/chrona`        | Temporal visualisation helpers and time/scenario UX support.                                   | Rust crate     |
| Metis Analytics        | `crates/metis`         | Analytics jobs (centrality, impact, TCO, rankings).                                             | Rust crate     |
| Continuum Orchestrator | `crates/continuum`     | Scheduler/connectors and background orchestration.                                             | Rust crate     |
| Mneme Core             | `crates/mneme`         | Bi-temporal storage engine (op log, facts, schema, projections) across SQL backends.          | Rust crate     |

For module-level internal design, see each `<module>/DESIGN.MD` (where present).

## Getting started

For a full walkthrough (prerequisites, setup, dev workflow, and issues helpers), see
`docs/GETTING-STARTED.MD`. The commands below are the most common entry points.

### Common commands (quick reference)

- Install deps: `corepack enable && pnpm install`
- Dev (Praxis workspace + Tauri host): see `docs/GETTING-STARTED.MD` for the recommended terminal layout.
- Lint/typecheck/test (TS): `pnpm run node:lint && pnpm run node:typecheck && pnpm run node:test`
- Rust checks: `pnpm run host:lint && pnpm run host:check`

See `docs/GETTING-STARTED.MD` and `docs/COMMANDS.MD` for the full list of pnpm commands used across
JS/TS and the Rust workspace.

## Key docs

- Suite design: `docs/DESIGN.MD`
- Architecture and layering: `ARCHITECTURE-BOUNDARY.MD`
- Coding standards: `docs/CODING-STANDARDS.MD`
- Testing strategy: `docs/TESTING-STRATEGY.MD`
- Agent guidance: `AGENTS.MD`
- Roadmap: `docs/ROADMAP.MD`

For contributing guidelines, see `CONTRIBUTING.MD` and `CODE-OF-CONDUCT.MD`. The license for this
repo is described in `LICENSE`.
