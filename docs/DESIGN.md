# Aideon Suite - Product & Design Overview

## Purpose

Describe the Aideon Suite at a product and conceptual design level: what problems it solves, how the
**time-first digital twin** is organised, and how the major modules fit together. Detailed
architecture and implementation design live in module `DESIGN.md` files and
`ARCHITECTURE-BOUNDARY.md`.

Aideon Suite is a **graph-native, time-first, local-first** digital twin platform for Enterprise
Architecture. It models strategy-to-execution as a graph, treats time/scenarios as first-class, and
runs primarily as a secure desktop app that can pivot to client-server mode.

## 1. Goals and principles

- **Graph-native:** Represent strategy, capabilities, services, processes, data, technology, and
  change as a single, queryable graph.
- **Time-first:** Use bi-temporal facts (valid time + asserted time), plan/actual layers, and
  scenarios to answer "what did we know, when?"
- **Local-first, cloud-ready:** Ship as a private desktop app with no open ports by default, with a
  clean pivot to remote/server mode via adapters.
- **Strict boundaries:** Renderer <-> Host <-> Engines communicate only via typed IPC and adapters;
  no backend logic in the renderer.
- **Security by default:** No renderer HTTP, least privilege, PII redaction on exports, and a
  hardened Tauri host.

See `ARCHITECTURE-BOUNDARY.md` for a deeper treatment of layering, adapters, and time semantics.

## 2. Modules in the suite

Aideon Suite is composed of several modules that share the same metamodel and time-first engine:

- **Aideon Desktop** - core desktop app (Aideon shell + Praxis workspace renderer + Tauri host + engines).
- **Aideon Praxis** - metamodel + task-oriented authoring + artefact execution + integrity/analytics orchestration.
- **Aideon Mneme** - bi-temporal persistence engine (op log, facts, schema, projections).
- **Aideon Chrona** - temporal visualisation over time/scenario slices.
- **Aideon Metis** - analytics engine for ranking, impact, and TCO.
- **Aideon Continuum** - orchestration, scheduling, and connectors.

The root `README.md` includes an "Aideon Suite modules" table with paths and responsibilities.
Internal structure and APIs for each module belong in that module's `README.md` and `DESIGN.md`.

## 3. Strategy-to-execution metamodel (summary)

Praxis provides a **three-layer model** that keeps the twin usable and analytically consistent:

- **Master types (anchors):** small, stable structural roles (Actor, Intent, Value, Capability,
  Execution, Technology, Structure, Change).
- **Domain types:** extensible business concepts that inherit from a master type.
- **Tasks:** user-facing operations that act on master-type roles and domain verbs (not raw graph
  primitives).

Praxis also defines **artefacts** as the primary user work products:

- Views, catalogues, matrices, maps, reports (and compositions/templates)
- Executed at a given time + scenario and returned in UI-ready shapes
- Consistent integrity scoring and explainability

The metamodel itself is published as **packages** with stable IDs, compiled into Mneme's
`MetamodelBatch`, and stored in Mneme's schema tables. For details, see
`crates/praxis/DESIGN.md` and `crates/mneme/DESIGN.md`.

## 4. Runtime architecture (suite-level)

At runtime, Aideon Suite is organised into three layers:

- **Renderer:** React/Tauri Praxis workspace renders artefact results using Aideon Design System
  components.
- **Host:** The Tauri-based Aideon Host manages windows, IPC commands, OS integration, and security
  capabilities.
- **Engines:** Rust engine crates (Praxis, Mneme, Chrona, Metis, Continuum) implement meaning,
  persistence, analytics, orchestration, and temporal visualisation.

Cross-cutting rules:

- Renderer never talks to databases or raw HTTP; it only calls the host via a typed bridge (for
  example the `praxisApi` wrapper under `app/AideonDesktop/src/adapters`).
- Engines expose traits and DTOs; the host selects local or remote implementations (future server
  mode) without changing renderer contracts.
- Desktop mode keeps all engine calls in-process with no open ports; server mode reuses the same
  DTOs over RPC.

The boundary and RPC design is covered in:

- `ARCHITECTURE-BOUNDARY.md`
- `docs/TAURI-CAPABILITIES.md`
- `docs/TAURI-CLIENT-SERVER-PIVOT.md`

## 5. UX surfaces and design system

The primary UX surface is the **Praxis workspace**: a node-based workspace that hosts widgets such
as views, catalogues, matrices, maps, charts, and timelines over the twin. Other surfaces
(dashboards, inspectors, reports) are built as widgets or panels within the same shell.

Design system decisions:

- React renderers use **Aideon Design System** (`app/AideonDesktop/src/design-system`), which wraps
  shadcn/ui and the React Flow UI registry into shared primitives and blocks.
- All React surfaces import from `@aideon/design-system/*` instead of talking directly to shadcn or
  React Flow.
- The legacy Svelte renderer has been removed; new work targets the React design system.

For UX and design details, see:

- `docs/UX-DESIGN.md` - UX goals, artefact UX contract, layouts, interaction principles.
- `docs/DESIGN-SYSTEM.md` - design system structure and usage.
- `app/AideonDesktop/DESIGN.md` - Aideon Desktop shell contract and workspace slots.

## 6. Data, integration, and automation

Data flow in Aideon Suite follows a "metamodel + facts" approach:

- **Persistence:** Mneme stores an operation log plus bi-temporal facts for nodes, edges, and
  properties, along with schema and projection tables for analytics.
- **Metamodel:** Praxis publishes metamodel packages into Mneme's schema tables (types, fields,
  edge rules), with stable IDs committed in source.
- **Artefacts:** Praxis stores and executes views, catalogues, matrices, maps, and reports as
  versioned artefacts with explicit schemas.
- **Integration:** Desktop mode provides read-only localhost APIs; server mode adds authenticated
  read/write APIs, CSV/XLSX import, and connectors (e.g., CMDB, cloud providers) via Continuum.
- **Automation:** Continuum orchestrates syncs, freshness checks, and connector jobs; results feed
  back into the twin and visualisations.

Detailed connector designs and SLOs should be captured in module-specific design docs (for example,
a future CMDB connector design under `crates/continuum/DESIGN.md`).

## 7. Where to go next

When working on Aideon Suite, use this document only as a **high-level map**. For deeper details:

- Architecture and boundaries: `ARCHITECTURE-BOUNDARY.md`
- Coding rules: `docs/CODING-STANDARDS.md`
- Testing approach: `docs/TESTING-STRATEGY.md`
- Tauri security and client-server pivot: `docs/TAURI-CAPABILITIES.md`,
  `docs/TAURI-CLIENT-SERVER-PIVOT.md`
- Module internals: `<module>/README.md` and `<module>/DESIGN.md`
- Canonical UX: `docs/UX-DESIGN.md`
