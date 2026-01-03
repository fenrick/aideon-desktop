# src/adapters – Aideon Suite module

_Flatten note: adapters now live at `app/AideonDesktop/src/adapters` within the desktop package; import via relative or `src`-rooted paths (no aliases)._

## Purpose

Praxis Adapters defines the TypeScript interfaces and contracts that form the UI boundary for
calling Host commands. Implementations are backend-agnostic and must not introduce backend
specifics into the renderer boundary.

Adapters call Host commands via invoke().

## Responsibilities

- Provide `GraphAdapter`/`MutableGraphAdapter` for time-sliced graph access.
- Surface meta-model information via `MetaModelProvider` so UIs can build forms dynamically.
- Define `StorageAdapter` for snapshot persistence.
- Define job DTOs for analytics and temporal jobs.
- Offer utilities like `ensureIsoDateTime` to normalise timestamp inputs.

## Relationships

- **Depends on:** TypeScript toolchain, `src/dtos` for shared DTOs.
- **Used by:** Praxis workspace, Aideon Desktop renderer, host adapter implementations.

## Running and testing

- Typecheck (suite-wide): `pnpm run node:typecheck`
- Tests (suite-wide, including adapters): `pnpm run node:test`

In module-specific tests, provide stub/fake implementations (e.g. `DevelopmentMemoryGraph`) to
exercise UI flows without a real backend.

## Design and architecture

Praxis Adapters encode the adapter pattern described in `ARCHITECTURE-BOUNDARY.md` for Graph,
Storage, and Worker boundaries. For suite-wide schema and meta-model details, see `docs/DESIGN.md`
and `docs/meta/README.md`.

## Related global docs

- Suite design and meta-model: `docs/DESIGN.md`, `docs/meta/README.md`
- Architecture and layering: `ARCHITECTURE-BOUNDARY.md`
- Coding standards: `docs/CODING-STANDARDS.md`
