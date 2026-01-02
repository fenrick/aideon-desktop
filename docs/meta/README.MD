# Metamodel Packages and Registry

## Purpose

Explain how the Aideon Suite metamodel is delivered and managed as **packages** rather than hard-
coded enums: where package definitions live, how they are published into Mneme, and how the Praxis
registry maps domain concepts to storage IDs.

Praxis owns the **meaning** of the twin (master types, domain types, verbs). Mneme owns the
**persistence** of schema data (types, fields, edge rules). This split keeps the system portable and
extensible without leaking storage details to the UI.

## Structure

Metamodel packages define:

- Master types (anchors) and their semantics
- Domain types (single inheritance) and defaults
- Domain verbs mapped to master edge semantics
- Field definitions and constraints

Packages are compiled by Praxis into a Mneme `MetamodelBatch`, which upserts:

- `aideon_types` / `aideon_type_extends`
- `aideon_fields` / `aideon_type_fields`
- `aideon_edge_type_rules`

All type/field/edge IDs are **stable UUIDs** committed in source to ensure long-term compatibility.

## Domain registry

Praxis maintains a registry that maps:

- `DomainTypeKey` -> Mneme `type_id`
- `DomainFieldKey` -> Mneme `field_id`
- `DomainVerb` -> Mneme `edge_type_id` + direction

The registry is used by task APIs so callers never need to handle storage IDs directly.

## Overrides and extensions

Additional packages can be installed per partition. Package evolution is versioned and can be
validated before publication. Praxis enforces single inheritance, endpoint constraints, and rule
compatibility when packages are applied.

## Storage notes

Mneme persists schema and edge semantics in its portable tables; no raw SQL is used in migrations or
runtime paths. See `crates/mneme/DESIGN.MD` for schema details.
