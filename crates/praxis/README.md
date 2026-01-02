# Praxis Engine - Aideon Suite module

## Purpose

Praxis is the **enterprise modelling and reasoning layer** for the Aideon digital twin. It defines
meaning (master types, domain types, verbs), exposes **task-oriented APIs** for authoring, and
returns **artefact results** (views, catalogues, matrices, maps, reports) that UIs can render
consistently across time and scenarios.

Praxis does **not** implement persistence or database logic. Mneme owns storage; Praxis owns
semantics, tasks, and integrity.

## Responsibilities

- Define and publish **metamodel packages** (master anchors + domain types + verbs) with stable IDs.
- Maintain a **domain registry** mapping domain keys and verbs to Mneme type/field/edge IDs.
- Provide **task-oriented authoring** APIs (create element, link, set attributes, move/contain).
- Execute **artefacts** (views, catalogues, matrices, maps, reports) with time/scenario context.
- Enforce **integrity rules** (directionality, logical/physical separation, spine completeness) and
  expose findings and integrity scores.
- Orchestrate **analytics** runs and explainability using Mneme projection edges (delegating compute
  to Metis when available).

## Relationships

- **Depends on:** Mneme for persistence, traversal, schema storage, and projection edges.
- **Works with:** Metis for analytics algorithms; Chrona for temporal visualisation.
- **Used by:** Aideon Host (Tauri IPC) and any server/CLI entry points.

## Design notes

- Task APIs operate on **domain concepts**, not storage IDs.
- Time/scenario context is explicit on every operation.
- No SQL or storage internals; all persistence goes through Mneme traits.
- No UI/Tauri code in this crate.

## Running and testing

- Rust tests (crate only): `cargo test -p aideon_praxis`
- Workspace Rust checks: `pnpm run host:lint && pnpm run host:check`

## Design and architecture

Praxis follows the task-oriented, master-type digital twin model described in
`crates/praxis/DESIGN.md`. Storage, time, and schema mechanics live in Mneme per
`crates/mneme/DESIGN.md`.
