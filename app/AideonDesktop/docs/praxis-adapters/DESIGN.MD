# PraxisAdapters – Design

## Purpose & scope

Praxis Adapters defines TypeScript interfaces for the renderer/host boundary (graph, storage,
worker). It is type-only and backend-agnostic.

## Allowed dependencies / frameworks

- TypeScript (strict), ESM modules.
- Shared DTOs from `src/dtos`.
- Light utilities only (e.g., type guards). No React, Tauri, DOM, or network libraries.

## Anti-goals

- No UI components, CSS, or renderer state.
- No host/engine logic or IPC wiring; implementations live elsewhere.
- No Node-specific APIs; keep packages portable across renderer and host contexts.

## Public surface

- Adapter interfaces: `GraphAdapter`, `MutableGraphAdapter`, `MetaModelProvider`, `StorageAdapter`,
  `WorkerClient` and worker job contracts.
- Shared types re-exported from `src/dtos` plus `ensureIsoDateTime` helper.
- Test fakes (e.g., `DevelopmentMemoryGraph`) for consumers.

## Evergreen notes

- Migrate any legacy Svelte-specific or CommonJS shims to ESM/React-era contracts.
- Prefer expanding adapter interfaces over adding ad-hoc IPC endpoints.
