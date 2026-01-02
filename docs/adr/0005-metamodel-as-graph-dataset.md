# ADR-0005: Meta-model as Graph Dataset (core-v1)

- Status: Accepted
- Date: 2025-11-17
- Related: `0003-adapter-boundaries.md`

## Context

Aideon Suite models a rich Enterprise Architecture meta-model (strategy, capabilities, services,
processes, data, technology, plateaus/plan events, etc.). We need that schema to be:

- Expressive enough to cover strategy-to-execution flows and analytics.
- Versioned and testable alongside regular data.
- Configurable per workspace or scenario without recompiling code.

Early docs described the meta-model only as prose. The engine stored opaque node/edge versions with
TODOs for schema validation; there was no clear, data-first source of truth.

## Decision

Treat the **meta-model as data**, expressed as a graph-style schema document and delivered as part
of the baseline dataset:

- The canonical schema lives at `docs/data/meta/core-v1.json` and is loaded into a
  `MetaModelRegistry` at runtime.
- The registry is seeded via normal commit/change-set APIs (see `meta_seed.rs`) so schema state is
  versioned like any other graph content.
- Overrides (tenant or scenario-specific) are also data: additional payloads alongside the baseline
  or commits on dedicated branches. The registry merges baseline + overrides into an effective
  schema.
- The host exposes the active schema via `temporal_metamodel_get`; renderers and tools consume it to
  drive forms, inspectors, and validation instead of hard-coding types.

## Rationale

- **Versioned and testable:** schema changes can be diffed, tested, and rolled out like any other
  dataset change.
- **Configurable:** tenants or scenarios can extend the meta-model without code changes, by
  providing additional schema data.
- **Aligned with graph semantics:** expressing the meta-model as a graph of types and relations
  matches how Praxis Engine and analytics crates operate.

## Consequences

- Schema validation must consult `MetaModelRegistry` for node/edge operations in Praxis Engine.
- Tools and UI components that need type information must ask the host for the active meta-model
  instead of embedding static enums.
- Detailed lists of element/relationship types belong in the dataset/meta docs and registry code,
  not in high-level suite docs.

## References

- `docs/meta/README.md` – explanation of meta-model payloads and overrides
- `docs/data/README.md` – baseline dataset and versioning
- `crates/praxis/src/meta_seed.rs` – code-first seeding of the schema
- `docs/DESIGN.md` – high-level design summary (now referencing this ADR instead of restating detail)
