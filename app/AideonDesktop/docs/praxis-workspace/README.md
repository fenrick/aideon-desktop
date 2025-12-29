# Praxis Workspace – Aideon Suite module

## Purpose

Praxis Workspace is the React/Tauri renderer surface for the **Aideon Desktop** app. It
hosts the primary workspace UI (widgets + time controls) and surfaces graph, catalogue, matrix, and
chart views over the time-first digital twin.

## Responsibilities

- Render the main workspace UI for Praxis via the workspace module contract.
- Manage widget composition, time/selection state, and template flows.
- Talk to the Tauri host via typed APIs (`app/AideonDesktop/src/workspaces/praxis/praxis-api.ts`).
- Consume shared UI primitives from `app/AideonDesktop/src/design-system`.
- Respect renderer boundaries (no direct filesystem/DB access; IPC only).

## Relationships

- **Depends on:** Aideon Design System, host IPC, shared DTOs.
- **Used by:** Aideon Desktop shell (React/Tauri renderer) via `WorkspaceModule` registration.

## Running and testing

- Dev (workspace renderer): `pnpm --filter @aideon/desktop run dev`
- Typecheck: `pnpm --filter @aideon/desktop run typecheck`
- JS/TS tests (suite-wide): `pnpm run node:test`

For multi-terminal dev workflow, see `docs/getting-started.md`.

## Design and architecture

Praxis Workspace is the primary renderer surface for the digital twin, built on React, XYFlow
(React Flow), and the shared design system. Internal layout, state, and widget composition are
described in `app/AideonDesktop/docs/praxis-workspace/DESIGN.md`. Suite-level architecture lives in
`Architecture-Boundary.md`.
