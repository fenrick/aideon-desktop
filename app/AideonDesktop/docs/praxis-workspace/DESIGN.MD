# Praxis Workspace – Internal Design

## Overview

Praxis Workspace is the React-based renderer surface for the Aideon Desktop app. It runs
inside the Aideon Desktop shell and projects the time-first digital twin into widgets such as
graph, catalogue, matrix, and timeline views.

Praxis exposes module components (`PraxisWorkspaceNavigation`, `PraxisWorkspaceToolbar`,
`PraxisWorkspaceContent`, `PraxisWorkspaceInspector`) that the Aideon Desktop shell renders via the
workspace module contract. `PraxisWorkspaceSurface` remains a chrome-free surface for standalone
embedding during tests or previews.

## Internal structure

- Workspace composition (templates, widgets, time panel) and UI state (selection, time cursor).
- State management for selection, filters, time cursor, and active template.
- Integration points to Aideon Design System blocks and React Flow-based canvas primitives.
- `PraxisWorkspaceProvider` owns shared state once; Navigation/Toolbar/Content/Inspector consume the
  provider so state is not duplicated across slots.
- Praxis navigation follows shadcn sidebar-08 as the base layout (inset sidebar + header layout),
  with nested rail behavior (sidebar-09), a team/workspace switcher, favorites, a collapsible file
  tree for scenarios (sidebar-11), and a right-side action popover (sidebar-10). The footer mirrors
  sidebar-08 with settings/feedback and an account switcher.

## Selection contract

- `PraxisWorkspaceSurface` accepts an optional `onSelectionChange` callback. The surface emits the
  current `SelectionState` whenever it changes so host shells or harnesses can observe selections.

## Time-first canvas behaviour

- Geometry persists per `asOf` (and optional `scenario`) via host IPC; renderer respects saved positions and only re-runs layout on demand.
- Auto-layout is user-triggered; default layout uses React Flow/ELK-compatible routines but must not override existing coordinates unless explicitly requested.
- Save/load flows go through typed adapters; no renderer-side storage beyond UI state.

## Data model and APIs

- Consumes DTOs from `src/dtos` (temporal snapshots/diffs, meta-model documents, job DTOs).
- Talks to the host exclusively via adapters defined in `src/adapters`.
- Treats the digital twin as read/write projections only; no private data store beyond UI state.

## Interactions

- Renderer ↔ Host: IPC commands exposed by Aideon Host (see `ARCHITECTURE-BOUNDARY.MD`).
- Renderer ↔ Engines: indirect, via host commands (no direct engine or DB access).
- Renderer ↔ Design system: components and tokens from `app/AideonDesktop/src/design-system`.

## UI patterns (golden example)

- **Golden vertical:** Time control + temporal panel + diff widgets (commit timeline, state-at, diff summary) use the current shadcn/React pattern. Use these as the template for new surfaces.
- **State management:** Co-locate state in hooks (`useTemporalPanel`) with return `[state, actions]`; avoid global singletons. Derive UI-ready state from DTOs; keep async side effects inside hooks.
- **Data fetching/IPC:** Use typed functions from `praxis-api.ts` that wrap Tauri IPC with consistent error handling. Keep IPC mapping inside the API layer, not components.
- **Loading/error/empty handling:** Components accept `loading`, `error`, and optional `empty` hints from their hook state and render design-system `Alert`/`Skeleton` patterns. Errors should be human-readable, not raw objects.
- **Composition:** Prefer shadcn blocks (cards, badges, buttons) and design-system primitives; avoid bespoke wrappers. Keep layout simple: card header, body, action row.
- **Testing:** Mirror the golden vertical tests: hook-level tests for state machines; component tests for rendering + interactions; IPC adapters mocked at the boundary.

## Constraints and invariants

- No backend logic or storage access in the renderer; IPC only.
- Twin is the source of truth; React state mirrors but never replaces it.
- Component and state boundaries should make it easy to plug in additional widgets (Chrona/Metis)
  without breaking existing flows.
