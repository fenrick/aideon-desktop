# Aideon Suite

Aideon Suite is a **time-first digital twin platform** delivered as a secure, offline-first desktop
application. It separates **meaning**, **storage**, and **runtime**, so the UI stays stable while
engines evolve behind typed boundaries.

## What makes it different

- **Time-first facts**: valid time + asserted time, Plan/Actual layers, scenario overlays.
- **Artefact-driven UX**: views, catalogues, matrices, maps, reports/pages.
- **Desktop as platform**: typed IPC, job orchestration, least privilege, no renderer HTTP.

## Modules

| Name                   | Path                | Responsibility                                                                    |
| ---------------------- | ------------------- | --------------------------------------------------------------------------------- |
| Aideon Desktop         | `app/AideonDesktop` | React renderer, design system, workspace surfaces, adapters, DTOs.                |
| Aideon Host            | `crates/desktop`    | Tauri runtime, IPC, capabilities, jobs, workspace lifecycle.                      |
| Praxis Engine          | `crates/praxis`     | Metamodel, task APIs, artefact execution, integrity, analytics orchestration.     |
| Mneme Core             | `crates/mneme`      | Op log, bi-temporal facts, schema-as-data, projections, processing.               |
| Metis Analytics        | `crates/metis`      | Analytics algorithms and ranking jobs.                                            |
| Chrona Visualisation   | `crates/chrona`     | Time/scenario UX primitives and temporal helpers.                                 |
| Continuum Orchestrator | `crates/continuum`  | Orchestration, scheduling, connectors.                                            |

## Central design docs

- Suite overview: `docs/DESIGN.md`
- Boundaries: `ARCHITECTURE-BOUNDARY.md`
- Host runtime: `crates/desktop/DESIGN.md`
- Praxis semantics: `crates/praxis/DESIGN.md`
- Mneme storage: `crates/mneme/DESIGN.md`
- UX contract: `docs/UX-DESIGN.md`

## Getting started

See `docs/GETTING-STARTED.md`. For available scripts, use `pnpm -w run`.
