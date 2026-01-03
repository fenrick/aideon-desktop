# Praxis DTOs – Aideon Suite module

_Flatten note: DTOs now live at `app/AideonDesktop/src/dtos` within the desktop package; import via relative or `src`-rooted paths (no aliases)._

## Purpose

Praxis DTOs defines shared TypeScript data-transfer objects used across Praxis workspace, Praxis
Desktop, and host adapters. It keeps IPC contracts consistent and strongly typed.

## Responsibilities

- DTOs must use camelCase across the boundary.
- Define DTOs for temporal state/diff snapshots, meta-model documents, and analytics/job payloads.
- Provide type-safe shapes for Praxis adapters (`src/adapters`) and React components.
- Keep frontend-facing types aligned with Rust DTOs from Mneme/Praxis Engine.

## Relationships

- **Depends on:** TypeScript toolchain and shared linting/typecheck setup.
- **Used by:** Praxis workspace, Aideon Desktop renderer, Praxis Adapters, and tests/fixtures.

## Running and testing

- Typecheck DTOs: `pnpm --filter @aideon/desktop run typecheck`
- Tests: `pnpm run node:test` (Vitest suite includes DTO tests)

DTOs are also typechecked as part of the suite-wide command: `pnpm run node:typecheck`.

## Design and architecture

For suite-wide schema and meta-model design, see `docs/DESIGN.md`, `docs/meta/README.md`, and
`docs/data/README.md`. If this package gains additional logic (codegen, versioning), extend
documentation in `app/AideonDesktop/docs/praxis-dtos/DESIGN.md`.
