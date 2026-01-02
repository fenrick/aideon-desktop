# PraxisDtos – Design

## Purpose & scope

Praxis DTOs holds shared TypeScript data-transfer objects for temporal state/diff snapshots,
meta-model documents, plan events, and worker job payloads. It keeps IPC and worker contracts
consistent across renderer, host, and engines.

## Allowed dependencies / frameworks

- TypeScript (strict), ESM modules.
- Minimal runtime code; DTOs are type-first. Helpers may use standard library only (e.g., date
  parsing in `ensureIsoDateTime`).
- No React, Tauri, DOM, or networking dependencies.

## Anti-goals

- No business logic, persistence, or IPC implementations.
- No UI components or styling.
- Avoid Node-specific APIs; DTOs must stay runtime-agnostic.

## Public surface

- Types exported from `temporal.ts`, `meta.ts`, `plan-event.ts`, and ISO helpers in `iso.ts`.
- Re-exports via `src/index.ts` for stable consumer imports.

## Evergreen notes

- Migrate any legacy loose `any`/`unknown` DTOs to explicit types.
- Keep DTOs aligned with Rust equivalents; prefer additive versioning over breaking changes.
