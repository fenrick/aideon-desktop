# src/adapters – Aideon Suite module

_Flatten note: adapters now live at `app/AideonDesktop/src/adapters` within the desktop package; import via relative or `src`-rooted paths (no aliases)._

## Purpose

Praxis Adapters defines the TypeScript interfaces and contracts that form the UI boundary for graph,
storage, and worker access. Implementations are backend-agnostic and must not introduce backend
specifics into the renderer boundary.

## Responsibilities

- Provide `GraphAdapter`/`MutableGraphAdapter` for time-sliced graph access.
- Surface meta-model information via `MetaModelProvider` so UIs can build forms dynamically.
- Define `StorageAdapter` for snapshot persistence.
- Define `WorkerClient` and related job DTOs for analytics and temporal jobs.
- Offer utilities like `ensureIsoDateTime` to normalise timestamp inputs.

## Relationships

- **Depends on:** TypeScript toolchain, `src/dtos` for shared DTOs.
- **Used by:** Praxis workspace, Praxis Desktop, host/worker adapter implementations.

## Running and testing

- Typecheck (suite-wide): `pnpm run node:typecheck`
- Tests (suite-wide, including adapters): `pnpm run node:test`

In module-specific tests, provide stub/fake implementations (e.g. `DevelopmentMemoryGraph`) to
exercise UI flows without a real backend.

## Design and architecture

Praxis Adapters encode the adapter pattern described in `ARCHITECTURE-BOUNDARY.MD` for Graph,
Storage, and Worker boundaries. For suite-wide schema and meta-model details, see `docs/DESIGN.MD`
and `docs/meta/README.MD`.

## Related global docs

- Suite design and meta-model: `docs/DESIGN.MD`, `docs/meta/README.MD`
- Architecture and layering: `ARCHITECTURE-BOUNDARY.MD`
- Coding standards: `docs/CODING-STANDARDS.MD`
