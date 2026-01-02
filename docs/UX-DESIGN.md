# Aideon Suite - Praxis UX Design Ground Rules (M0 -> M1)

## Purpose

Define UX goals, interaction patterns, and layout decisions for the Praxis desktop module within
Aideon Suite. This document guides how the canvas, sidebar, toolbars, and artefact surfaces should
behave and look, independent of specific implementation details in React.

Date: 2025-10-28
Status: Draft (implements M0 baseline; informs M1 scope)

> **Renderer migration:** Historic UX notes reference Svelte components. Use them as conceptual
> guidance only; new implementation work must target the React + React Flow + shadcn/ui stack.

## Goals

- Feel native and respectful on each OS while retaining a coherent visual language.
- Keep menus and window chrome platform-native where possible; keep the in-content component kit
  small, fast, and token-driven.
- Ensure all user-facing outputs are grounded in Praxis artefacts (views, catalogues, matrices,
  maps, reports) and time/scenario context.

## 1) UX contract for Praxis artefacts

Praxis defines **semantic outputs**, not pixels. The UI must render artefact results and diagram
specs without inventing new semantics.

Required artefact surfaces:

- Views (graph subgraphs and narratives)
- Catalogues (lists with grouping, filters, drill-down)
- Matrices (coverage and relationship analysis)
- Maps (capability, journey, application landscape)
- Reports (composed artefacts + commentary)

All artefacts are executed with explicit **time context** and optional **scenario**. UI must surface
those parameters and pass them with every request.

## 2) Page model and composition

Artefacts render into a **Page Model** that supports composition and export:

- Page layout is declarative (`single`, `two-column`, `grid`, `freeform`).
- Sections embed artefacts with layout hints and export modes.
- Headers/footers are data-driven and include time/scenario context.

The UI must support pages as a first-class container for dashboards and exports.

## 3) Diagram model (shared visual language)

All visualisations are instances of a **single diagram model**:

- Visual nodes and edges
- Containers/groups
- Overlays and legends
- Layout hints (layered, hierarchy, swimlane, matrix)

UI responsibilities:

- Render diagram specs consistently across artefacts.
- Honor overlays (heatmaps, badges, categories).
- Support drill-down and selection on nodes/edges/containers.

## 4) Time and scenario UX

- Time controls are always visible in the primary workspace.
- Time context includes valid time + layer (Plan/Actual). Scenario is optional.
- UI must allow time scrubbing and scenario switching without reloading the shell.
- Time-aware visuals should show change annotations (added/removed/changed).

## 5) Matrix and catalogue UX

**Matrices**

- Sparse grid rendering with sticky headers and totals.
- Heatmap overlays per cell.
- Cell click opens drill-down (edges, paths, missing links).

**Catalogues**

- Column sorting, faceted filtering, and grouping.
- Inline indicators for integrity warnings and coverage metrics.
- Pivot actions to open related views/maps/matrices.

## 6) Interaction and drill-down

- Drill-down is universal: given artefact + selection, open element summaries and suggested tasks.
- Reverse navigation is required (matrix cell -> view -> element editor).
- Selection is a shared contract between widgets, inspector, and actions.

## 7) Window and chrome

- Menus: use platform-native menus (Tauri Menu API). No HTML top nav.
- Titlebar: default to system window chrome. Allow optional frameless+drag regions per OS in a
  later design update.
- Effects: optional per-OS vibrancy/mica when available; keep it opt-in.

## 8) Component strategies and layout

- Layout: toolbar-driven shell with top toolbar, left sidebar, main content, and bottom status bar
  for connection/health and short messages.
- Toolbars: grouped icon buttons similar to ribbons. Idle uses outline; active uses filled variant
  and picks up the accent color from tokens.
- Sidebar: tree/navigation for catalogues, metamodel, and views. Use design-system sidebar proxies
  (inset + nested sections + file tree).
- Content: detail panels for selected items (elements, catalogues, matrices, maps).

### React/shadcn layering principles

- Use vanilla shadcn components in their default theme so upstream updates remain easy to adopt.
- Layer the Praxis design system on top by composing blocks (cards, inspectors, toolbars, command
  palettes) out of shadcn primitives and tokens, rather than forking primitives.
- Sync primitives before editing with `pnpm --filter @aideon/desktop run components:refresh`.
- Build UX screens from reusable blocks first; promote patterns to blocks once they appear twice.
- Keep Tailwind utility usage scoped inside the blocks.
- React Flow UI registry components should be consumed via proxies, not modified in place.

## 9) Theming and tokens

- Tokens live in `app/AideonDesktop/src/design-system/src/styles/globals.css` and are consumed via
  `@aideon/design-system` wrappers; platform accents are previewed in the Praxis workspace Style
  Guide window.
- Primary token: `--color-accent` drives primary buttons, focus rings, selected state.
- Platform dev-preview:
  - mac: `--color-accent: #0a84ff`
  - win: `--color-accent: #0078d4`
  - linux: `--color-accent: #16a34a`
- Future: read system theme and accent where APIs permit; allow user override.

## 10) Interaction and accessibility

- Keyboard: Cmd on macOS, Ctrl on Windows/Linux. Define accelerators in the native menu.
- Focus rings: use tokenized visible focus; avoid removing outlines.
- Tabs: close affordance must be keyboard-operable.
- Split panes: mouse drag; keyboard adjustment optional where a11y rules allow.

## 11) Dev experience

- A Style Guide window (Debug -> UI Style Guide) previews tokens and controls; includes a platform
  toggle (auto/mac/win/linux) to preview platform accents and token overrides.
- The Style Guide does not ship to production.

## 12) Implementation notes

- Keep renderer code free of backend logic; use typed IPC only.
- Keep per-OS tweaks purely in CSS tokens or shallow component wrappers; avoid forking components
  by OS.
- The Aideon Desktop shell contract and workspace slot model are defined in
  `app/AideonDesktop/DESIGN.md` and should be treated as authoritative for shell layout decisions.

## 13) Acceptance

- Switching the Style Guide platform toggle updates `--color-accent` live; primary buttons,
  switches, and focus rings visibly change.
- Menus are native on all three OSes.
- Lint/typecheck/tests pass; no "non reactive update" warnings.
- Praxis artefacts render with consistent diagram semantics and drill-down.
