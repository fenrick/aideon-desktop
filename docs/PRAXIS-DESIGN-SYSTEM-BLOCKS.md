# Praxis Workspace - Design System Block Audit

## Purpose

Capture the layered blocks that now live in `app/AideonDesktop/src/design-system`, confirm Praxis
workspace consumes them without forking vanilla shadcn/reactflow components, and highlight gaps
left in the React migration. This is an implementation audit for the Praxis workspace module, not a
general design system specification.

Date: 2025-11-16

## Compliance summary

- All shadcn/ui and React Flow UI primitives sit inside
  `app/AideonDesktop/src/design-system/src/ui` or `src/reactflow`; the only `@radix-ui/*` imports in
  the workspace reside there. Refreshing all primitives is a single command scoped to the package:
  `pnpm --filter @aideon/desktop run components:refresh` (pulls shadcn defaults plus the React Flow
  UI registry entries such as `@reactflow/base-node`, `@reactflow/node-tooltip`,
  `@reactflow/node-search`, `@reactflow/animated-svg-edge`).
- `components.json` in the design-system package tracks both registries (shadcn default + React
  Flow) so we can add future sources without touching app bundles. No renderer edits are required
  when the registries are refreshed.
- Vanilla-first blocks now live under `app/AideonDesktop/src/design-system/src/blocks` (panel,
  toolbar, sidebar, modal). They proxy the primitives so UX surfaces (forms, toolbars, modals,
  sidebars) can be assembled from consistent Lego pieces while keeping upstream code refreshable.
- Praxis workspace imports everything through `src/design-system`. Example: the `TimeControlPanel`
  block now uses `Panel`, `PanelField`, and `PanelToolbar` rather than pulling shadcn cards
  directly, proving the proxy layer is working.
- React Flow canvas nodes/edges (`PraxisNode`, `TimelineEdge`, `NodeSearchDialog`) are defined in the
  design system package and consume the vanilla React Flow UI exports untouched; Praxis-specific
  styling/tokens happen only in the proxy wrappers.

## Block inventory

### Shared design-system blocks

| Block                                                                                                                | File                                                         | Built from                                 | Notes                                                                                                                                                                             |
| -------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ | ------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Panel stack (`Panel`, `PanelHeader`, `PanelTitle`, `PanelDescription`, `PanelContent`, `PanelField`, `PanelToolbar`) | `app/AideonDesktop/src/design-system/src/blocks/panel.tsx`   | shadcn `Card` primitives + Tailwind tokens | Canonical form block for cards, inspectors, and control panels. `TimeControlPanel` is the first adopter; roadmap is to migrate the remaining dashboard cards and inspector views. |
| Modal shell (`Modal`, `ModalContent`, `ModalHeader`, `ModalTitle`, `ModalDescription`, `ModalFooter`)                | `app/AideonDesktop/src/design-system/src/blocks/modal.tsx`   | shadcn `Dialog` primitives                 | Provides default padding/radius for dialogs so command palette, exports, and confirmation prompts stay consistent. Ready for adoption as command/search modals are refreshed.     |
| Toolbar (`Toolbar`, `ToolbarSection`, `ToolbarSeparator`)                                                            | `app/AideonDesktop/src/design-system/src/blocks/toolbar.tsx` | Tailwind layout + shadcn tokens            | Used for canvas/tool controls; aligns with UX ask for vanilla shadcn blocks assembled into reusable toolbars. Pending wiring into the React shell toolbar.                        |
| Sidebar (`SidebarShell`, `SidebarSection`, `SidebarHeading`, `SidebarNav`)                                           | `app/AideonDesktop/src/design-system/src/blocks/sidebar.tsx` | Semantic `aside` + shadcn tokens           | Provides the baseline for navigation and catalog inspectors; will replace bespoke sidebar patterns in the Praxis workspace once search wiring lands.                              |

### Praxis workspace adoption snapshot

| Surface                                                                                      | Consumed blocks                                                         | Status                                                                                               |
| -------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Time control panel (`app/AideonDesktop/src/canvas/components/blocks/time-control-panel.tsx`) | `Panel`, `PanelField`, `PanelToolbar`, shadcn `Select`/`Slider` proxies | OK Migrated to the shared panel block; demonstrates field + helper usage and action toolbar.         |
| Command palette (`components/blocks/temporal-command-menu.tsx`)                              | `Modal` (planned), shadcn `Command`                                     | Planned Next step is to swap in `Modal` wrappers so dialogs inherit the shared chrome.               |
| Sidebar shell (`components/app-sidebar.tsx`)                                                 | `SidebarShell`, `SidebarHeading` (planned)                              | Planned Currently bespoke; target is to adopt the sidebar block once search shortcuts port to React. |
| Canvas toolbar + widget toolbars                                                             | `Toolbar`, `ToolbarSection` (planned)                                   | Planned Will replace bespoke flex rows as soon as timeline/activity tabs move to React.              |
| React Flow nodes/edges                                                                       | `PraxisNode`, `TimelineEdge`, `NodeSearchDialog` from design system     | OK Already consuming vanilla React Flow UI registry components via proxies.                          |

## Follow-ups (implementation-only)

- Convert the remaining dashboard cards (`time-cursor`, `worker-health`, `selection-inspector`,
  etc.) to the shared `Panel` stack so future apps get the same block palette.
- Switch the command palette + context modals to the `Modal` shell and move the React sidebar onto
  the `Sidebar` block before removing remaining legacy UI fragments.

This file is an implementation audit for the Praxis workspace module; the canonical description of
UX and design system principles lives in `docs/UX-DESIGN.md` and `docs/DESIGN-SYSTEM.md`.
