# Meta-Model Artifacts

## Purpose

Explain how the Aideon Suite meta-model is delivered and managed as data rather than code: where the
baseline schema lives, how it is seeded into the twin, and how overrides are applied. This is the
primary reference for meta-model payloads and registry behaviour.

The **baseline schema** is a payload inside the dataset (`docs/data/meta/core-v1.json`) so it can be
versioned, imported, and branched along with the rest of the graph. The renderer and host load that
document through `MetaModelRegistry`, so runtime validation and UI forms always stay data-driven.

## Structure

The payload expresses the schema using a graph-style hierarchy of object types where each node
represents an ArchiMate concept (`Capability`, `Application`, `PlanEvent`, etc.). Each type
includes attribute definitions, enum constraints, and allowed relationships to other types. The
host treats the payload as a commit-level artifact, and future configuration screens will let
stewards build these structures interactively (nesting attributes, grouping relationships, and
linking effects) instead of editing JSON manually.

## Code-first seeding

`crates/praxis/src/meta_seed.rs` contains the code representation of this schema. When a new
database is created, `PraxisEngine::ensure_seeded` calls `meta_model_seed_change_set` so the
meta-model and relationship descriptors are built as regular nodes/edges via the commit APIs
instead of reading the JSON at runtime. This dog-foots the same APIs the renderer uses and keeps
the schema aligned with the baseline dataset without introducing JSON parsing hooks in production.

## Overrides

Overrides live alongside the baseline payload, such as `.praxis/meta/<tenant>.json` or as
scenario-specific commits. The registry merges them in order, so the desktop renderer always
receives the effective schema via `temporal_metamodel_get`. Since they are just data, adding new
object types or constraints is handled by extending the dataset rather than editing Rust.

## SeaORM persistence

The SQLite persistence layer now runs through SeaORM/SeaQuery 1.1.19 located in `crates/mneme`.
Startup migrations create the `commits`, `refs`, `snapshots`, and readonly `metis_events` tables and
the meta-model dataset is seeded via those tables so analytics can consume both the graph and the
flattened Metis view without embedding schema logic in the renderer.
